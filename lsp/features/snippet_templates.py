"""Context-aware snippet templates for LSP completion.

Generates VS Code snippet-format completion items that adapt to the current
formatter config (indent style, brace style), dialect, enclosing iRules event,
variables in scope, and events already declared in the file.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass

from lsprotocol import types

from core.formatting.config import BraceStyle

# Context passed to every generator


@dataclass(frozen=True)
class SnippetContext:
    """Immutable context for snippet generation."""

    dialect: str  # "tcl", "f5-irules", "f5-iapps", etc.
    brace_style: BraceStyle  # K_AND_R
    indent_unit: str  # e.g. "    " or "\t"
    current_event: str | None  # enclosing when-event, or None
    file_events: frozenset[str]  # all when-events declared in the file
    scope_vars: list[str]  # variable names accessible at cursor
    partial: str  # text typed so far


# Template registry


@dataclass(frozen=True)
class SnippetTemplate:
    """Metadata for a single snippet template."""

    prefix: str
    label: str
    detail: str
    generator: Callable[[SnippetContext], str | None]
    dialects: frozenset[str] | None = None  # None = all dialects
    requires_top_level: bool = False  # only outside any when block


_TEMPLATES: list[SnippetTemplate] = []


def _register(
    prefix: str,
    label: str,
    detail: str,
    generator: Callable[[SnippetContext], str | None],
    *,
    dialects: frozenset[str] | None = None,
    requires_top_level: bool = False,
) -> None:
    _TEMPLATES.append(
        SnippetTemplate(
            prefix=prefix,
            label=label,
            detail=detail,
            generator=generator,
            dialects=dialects,
            requires_top_level=requires_top_level,
        )
    )


# Helpers

_IRULES = frozenset({"f5-irules"})


def _open_brace(ctx: SnippetContext) -> str:
    """Opening-brace fragment respecting brace_style."""
    return "{"


def _var_choices(ctx: SnippetContext, tabstop: int, default: str) -> str:
    """Build a snippet placeholder with scope-variable choices, or a plain default."""
    if ctx.scope_vars:
        escaped = [v.replace(",", "\\,").replace("|", "\\|") for v in ctx.scope_vars[:10]]
        choices = ",".join(f"\\${v}" for v in escaped)
        return f"${{{tabstop}|{choices}|}}"
    return f"${{{tabstop}:{default}}}"


# Tcl core template generators


def _gen_proc(ctx: SnippetContext) -> str:
    ob = _open_brace(ctx)
    i = ctx.indent_unit
    return f"proc ${{1:name}} {{${{2:args}}}} {ob}\n{i}$0\n}}"


def _gen_namespace(ctx: SnippetContext) -> str:
    ob = _open_brace(ctx)
    i = ctx.indent_unit
    return f"namespace eval ${{1:::ns}} {ob}\n{i}$0\n}}"


def _gen_package(ctx: SnippetContext) -> str:
    ob = _open_brace(ctx)
    i = ctx.indent_unit
    return (
        f"package require Tcl ${{1:8.6}}\n"
        f"package provide ${{2:pkgname}} ${{3:1.0}}\n"
        f"\n"
        f"namespace eval ${{4:::${{2}}}} {ob}\n"
        f"{i}namespace export ${{5:*}}\n"
        f"}}\n"
        f"\n"
        f"$0"
    )


def _gen_class(ctx: SnippetContext) -> str:
    ob = _open_brace(ctx)
    i = ctx.indent_unit
    ii = ctx.indent_unit * 2
    return (
        f"oo::class create ${{1:ClassName}} {ob}\n"
        f"{i}constructor {{${{2:args}}}} {ob}\n"
        f"{ii}${{3:# init}}\n"
        f"{i}}}\n"
        f"\n"
        f"{i}method ${{4:methodName}} {{${{5:args}}}} {ob}\n"
        f"{ii}$0\n"
        f"{i}}}\n"
        f"}}"
    )


def _gen_if(ctx: SnippetContext) -> str:
    ob = _open_brace(ctx)
    i = ctx.indent_unit
    return f"if {{${{1:condition}}}} {ob}\n{i}${{2:# then}}\n}} else {ob}\n{i}$0\n}}"


def _gen_foreach(ctx: SnippetContext) -> str:
    ob = _open_brace(ctx)
    i = ctx.indent_unit
    list_ph = _var_choices(ctx, 2, "listVar")
    return f"foreach ${{1:item}} {list_ph} {ob}\n{i}$0\n}}"


def _gen_for(ctx: SnippetContext) -> str:
    ob = _open_brace(ctx)
    i = ctx.indent_unit
    return f"for {{set ${{1:i}} 0}} {{\\$${{1:i}} < ${{2:10}}}} {{incr ${{1:i}}}} {ob}\n{i}$0\n}}"


def _gen_switch(ctx: SnippetContext) -> str:
    ob = _open_brace(ctx)
    i = ctx.indent_unit
    ii = ctx.indent_unit * 2
    return (
        f"switch -- ${{1:value}} {ob}\n"
        f"{i}${{2:pattern}} {ob}\n"
        f"{ii}${{3:# body}}\n"
        f"{i}}}\n"
        f"{i}default {ob}\n"
        f"{ii}$0\n"
        f"{i}}}\n"
        f"}}"
    )


def _gen_catch(ctx: SnippetContext) -> str:
    ob = _open_brace(ctx)
    i = ctx.indent_unit
    return (
        f"if {{[catch {ob}\n"
        f"{i}${{1:# risky call}}\n"
        f"}} result opts]}} {ob}\n"
        f"{i}puts stderr \\$result\n"
        f"{i}return -code error -options \\$opts \\$result\n"
        f"}}\n"
        f"$0"
    )


def _gen_try(ctx: SnippetContext) -> str:
    ob = _open_brace(ctx)
    i = ctx.indent_unit
    return (
        f"try {ob}\n"
        f"{i}${{1:# body}}\n"
        f"}} trap {{${{2:TCL}} ${{3:*}}}} {{result opts}} {ob}\n"
        f"{i}$0\n"
        f"}}"
    )


def _gen_dict_for(ctx: SnippetContext) -> str:
    ob = _open_brace(ctx)
    i = ctx.indent_unit
    dict_ph = _var_choices(ctx, 3, "dictVar")
    return f"dict for {{${{1:key}} ${{2:value}}}} {dict_ph} {ob}\n{i}$0\n}}"


# iRules template generators


def _gen_rule_init(ctx: SnippetContext) -> str | None:
    if "RULE_INIT" in ctx.file_events:
        return None
    ob = _open_brace(ctx)
    i = ctx.indent_unit
    return f"when RULE_INIT {ob}\n{i}$0\n}}"


def _gen_http_request(ctx: SnippetContext) -> str | None:
    if "HTTP_REQUEST" in ctx.file_events:
        return None
    ob = _open_brace(ctx)
    i = ctx.indent_unit
    ii = ctx.indent_unit * 2
    return (
        f"when HTTP_REQUEST {ob}\n"
        f"{i}set host [string tolower [HTTP::host]]\n"
        f"{i}set path [HTTP::path]\n"
        f"{i}set uri [HTTP::uri]\n"
        f"\n"
        f"{i}if {{\\$debug}} {ob}\n"
        f'{ii}log local0.debug "HTTP_REQUEST host=\\$host path=\\$path"\n'
        f"{i}}}\n"
        f"\n"
        f"{i}$0\n"
        f"}}"
    )


def _gen_redirect_https(ctx: SnippetContext) -> str | None:
    if "HTTP_REQUEST" in ctx.file_events:
        return None
    ob = _open_brace(ctx)
    i = ctx.indent_unit
    ii = ctx.indent_unit * 2
    return (
        f"when HTTP_REQUEST {ob}\n"
        f"{i}if {{[TCP::local_port] == 80}} {ob}\n"
        f'{ii}HTTP::redirect "https://[HTTP::host][HTTP::uri]"\n'
        f"{ii}return\n"
        f"{i}}}\n"
        f"\n"
        f"{i}$0\n"
        f"}}"
    )


def _gen_collect_release(ctx: SnippetContext) -> str | None:
    has_req = "HTTP_REQUEST" in ctx.file_events
    has_data = "HTTP_REQUEST_DATA" in ctx.file_events
    if has_req and has_data:
        return None
    ob = _open_brace(ctx)
    i = ctx.indent_unit
    ii = ctx.indent_unit * 2
    parts: list[str] = []
    if not has_req:
        parts.append(
            f"when HTTP_REQUEST {ob}\n"
            f'{i}if {{[HTTP::method] eq "POST"}} {ob}\n'
            f"{ii}HTTP::collect ${{1:1024}}\n"
            f"{ii}return\n"
            f"{i}}}\n"
            f"\n"
            f"{i}${{2:# non-body handling}}\n"
            f"}}"
        )
    if not has_data:
        parts.append(
            f"when HTTP_REQUEST_DATA {ob}\n"
            f"{i}set payload [HTTP::payload]\n"
            f"{i}HTTP::release\n"
            f"{i}$0\n"
            f"}}"
        )
    return "\n\n".join(parts) if parts else None


def _gen_class_lookup(ctx: SnippetContext) -> str | None:
    if "HTTP_REQUEST" in ctx.file_events:
        return None
    ob = _open_brace(ctx)
    i = ctx.indent_unit
    ii = ctx.indent_unit * 2
    return (
        f"when HTTP_REQUEST {ob}\n"
        f"{i}set host [string tolower [HTTP::host]]\n"
        f"{i}set pool_name [class match -value \\$host equals ${{1:host_to_pool_dg}}]\n"
        f'{i}if {{\\$pool_name ne ""}} {ob}\n'
        f"{ii}pool \\$pool_name\n"
        f"{ii}return\n"
        f"{i}}}\n"
        f"\n"
        f"{i}$0\n"
        f"}}"
    )


# Register all templates

_register("tcl-proc", "Tcl Proc", "Create a Tcl procedure", _gen_proc)
_register("tcl-namespace", "Namespace Eval", "Create a namespace eval block", _gen_namespace)
_register(
    "tcl-package", "Package Boilerplate", "Create package provide/require boilerplate", _gen_package
)
_register("tcl-class", "OO Class", "Create an oo::class", _gen_class)
_register("tcl-if", "If Else", "Create braced if/else block", _gen_if)
_register("tcl-foreach", "Foreach", "Create a foreach loop", _gen_foreach)
_register("tcl-for", "For Loop", "Create a for loop with braced expressions", _gen_for)
_register("tcl-switch", "Switch", "Create a switch block with -- option terminator", _gen_switch)
_register(
    "tcl-catch",
    "Catch with Result",
    "Create a catch pattern that preserves result and options",
    _gen_catch,
)
_register("tcl-try", "Try Trap", "Create a try/trap block", _gen_try)
_register("tcl-dict-for", "Dict For", "Iterate key/value pairs in a dict", _gen_dict_for)

_register(
    "irule-rule-init",
    "iRule RULE_INIT",
    "Initialise iRule static state",
    _gen_rule_init,
    dialects=_IRULES,
    requires_top_level=True,
)
_register(
    "irule-http-request",
    "iRule HTTP_REQUEST",
    "HTTP_REQUEST handler with safe defaults",
    _gen_http_request,
    dialects=_IRULES,
    requires_top_level=True,
)
_register(
    "irule-redirect-https",
    "iRule Redirect HTTPS",
    "Redirect HTTP traffic to HTTPS",
    _gen_redirect_https,
    dialects=_IRULES,
    requires_top_level=True,
)
_register(
    "irule-collect-release",
    "iRule Collect/Release",
    "Collect payload and release in HTTP_REQUEST_DATA",
    _gen_collect_release,
    dialects=_IRULES,
    requires_top_level=True,
)
_register(
    "irule-class-lookup",
    "iRule Data Group Lookup",
    "Data-group lookup and route",
    _gen_class_lookup,
    dialects=_IRULES,
    requires_top_level=True,
)


# Public API


def get_snippet_completions(ctx: SnippetContext) -> list[types.CompletionItem]:
    """Return context-aware snippet completion items."""
    items: list[types.CompletionItem] = []
    for tmpl in _TEMPLATES:
        # Dialect filter
        if tmpl.dialects is not None and ctx.dialect not in tmpl.dialects:
            continue
        # Top-level guard
        if tmpl.requires_top_level and ctx.current_event is not None:
            continue
        # Prefix filter
        if ctx.partial and not tmpl.prefix.startswith(ctx.partial):
            continue
        # Generate snippet body
        body = tmpl.generator(ctx)
        if body is None:
            continue
        items.append(
            types.CompletionItem(
                label=tmpl.label,
                kind=types.CompletionItemKind.Snippet,
                detail=tmpl.detail,
                insert_text=body,
                insert_text_format=types.InsertTextFormat.Snippet,
                filter_text=tmpl.prefix,
                sort_text=f"Z0_{tmpl.prefix}",
            )
        )
    return items
