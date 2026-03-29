#!/usr/bin/env python3
"""Generate C++ constexpr command descriptors from the Python registry.

Reads the Python CommandSpec registry and emits C++ files compatible with
the native/include/tcl_lsp/registry/command_desc.hpp types.

Usage:
    python scripts/registry/generate_native_registry.py
    python scripts/registry/generate_native_registry.py --validate
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from core.commands.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    SubCommand,
)
from core.commands.registry.signatures import ArgRole, Arity

# Lazy imports for side_effects and types — they may pull in heavy modules.
_SideEffect = None
_TclType = None
_StorageScope = None
_ConnectionSide = None
_SideEffectTarget = None
_TaintColour = None


def _ensure_side_effects():
    global _SideEffect, _SideEffectTarget, _ConnectionSide, _StorageScope
    if _SideEffect is None:
        from core.compiler.side_effects import (
            ConnectionSide,
            SideEffect,
            SideEffectTarget,
            StorageScope,
        )
        _SideEffect = SideEffect
        _SideEffectTarget = SideEffectTarget
        _ConnectionSide = ConnectionSide
        _StorageScope = StorageScope


def _ensure_types():
    global _TclType
    if _TclType is None:
        from core.compiler.types import TclType
        _TclType = TclType


def _ensure_taint():
    global _TaintColour
    if _TaintColour is None:
        from core.commands.registry.taint_hints import TaintColour
        _TaintColour = TaintColour


# ─── C++ identifier helpers ────────────────────────────────────────


def _safe_ident(name: str) -> str:
    """Convert a command name to a safe C++ identifier."""
    # Use double underscore for :: to avoid collisions with literal underscores
    s = name.replace("::", "__").replace("-", "_").replace("+", "_plus").replace(".", "_dot")
    if s and s[0].isdigit():
        s = "_" + s
    return s


def _c_string_escape(s: str) -> str:
    """Escape a string for C++ string literal."""
    return (
        s.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
    )


# ─── Mapping tables ────────────────────────────────────────────────

DIALECT_FLAG_MAP = {
    "tcl8.4": "TCL84",
    "tcl8.5": "TCL85",
    "tcl8.6": "TCL86",
    "tcl9.0": "TCL90",
    "f5-irules": "IRULES",
    "f5-iapps": "IAPPS",
    "tmsh": "TMSH",
    "f5-tmsh": "TMSH",
    "f5-bigip": "IAPPS",
    "tk": "TK",
    "expect": "EXPECT",
    "synopsys-eda-tcl": "EDA_SYNOPSYS",
    "cadence-eda-tcl": "EDA_CADENCE",
    "xilinx-eda-tcl": "EDA_XILINX",
    "intel-quartus-eda-tcl": "EDA_QUARTUS",
    "mentor-eda-tcl": "EDA_MENTOR",
}

ARGROLE_MAP = {
    ArgRole.BODY: "BODY",
    ArgRole.EXPR: "EXPR",
    ArgRole.VAR_NAME: "VAR_NAME",
    ArgRole.VAR_READ: "VAR_READ",
    ArgRole.PARAM_LIST: "PARAM_LIST",
    ArgRole.NAME: "NAME",
    ArgRole.PATTERN: "PATTERN",
    ArgRole.OPTION: "OPTION",
    ArgRole.VALUE: "VALUE",
    ArgRole.SUBCOMMAND: "SUBCOMMAND",
    ArgRole.OPTION_TERMINATOR: "OPTION_TERMINATOR",
    ArgRole.CHANNEL: "CHANNEL",
    ArgRole.INDEX: "INDEX",
}
# FORMAT_STRING and LOOP_VAR exist only in C++ — Python doesn't have them.

TCLTYPE_MAP: dict[str, str] = {}  # populated lazily

FORMKIND_MAP = {
    FormKind.DEFAULT: "DEFAULT",
    FormKind.GETTER: "GETTER",
    FormKind.SETTER: "SETTER",
}

FORMATTYPE_MAP = {
    "sprintf": "SPRINTF",
    "clock": "CLOCK",
    "binary": "BINARY",
    "regsub": "REGSUB",
}

PATTERNTYPE_MAP = {
    "glob": "GLOB",
    "regex": "REGEX",
}

SIDE_EFFECT_TARGET_MAP = {
    "VARIABLE": "VARIABLE",
    "SESSION_TABLE": "SESSION_TABLE",
    "PERSISTENCE_TABLE": "PERSISTENCE_TABLE",
    "DATA_GROUP": "DATA_GROUP",
    "HTTP_HEADER": "HTTP_HEADER",
    "HTTP_BODY": "HTTP_BODY",
    "HTTP_STATUS": "HTTP_STATUS",
    "HTTP_URI": "HTTP_URI",
    "HTTP_COOKIE": "HTTP_COOKIE",
    "HTTP_METHOD": "HTTP_METHOD",
    "HTTP2_STATE": "HTTP2_STATE",
    "RESPONSE_COMMIT": "RESPONSE_COMMIT",
    "CONNECTION_CONTROL": "CONNECTION_CONTROL",
    "TCP_STATE": "TCP_STATE",
    "SSL_STATE": "SSL_STATE",
    "SIP_STATE": "SIP_STATE",
    "DNS_STATE": "DNS_STATE",
    "FTP_STATE": "FTP_STATE",
    "DIAMETER_STATE": "DIAMETER_STATE",
    "RADIUS_STATE": "RADIUS_STATE",
    "MR_STATE": "MR_STATE",
    "STREAM_STATE": "STREAM_STATE",
    "LOG_OUTPUT": "LOG_OUTPUT",
    "FILE_IO": "FILE_IO",
    "INTERP_STATE": "INTERP_STATE",
    "UNKNOWN": "UNKNOWN",
}

CONNECTION_SIDE_MAP = {
    "CLIENT": "CLIENT",
    "SERVER": "SERVER",
    "BOTH": "BOTH",
    "GLOBAL": "GLOBAL",
    "NONE": "NONE",
}

STORAGE_SCOPE_MAP = {
    "PROC_LOCAL": "PROC_LOCAL",
    "NAMESPACE": "NAMESPACE",
    "GLOBAL": "GLOBAL",
    "UPVAR": "UPVAR",
    "EVENT": "EVENT",
    "CONNECTION": "CONNECTION",
    "STATIC": "STATIC",
    "SESSION_TABLE": "SESSION_TABLE",
    "PERSISTENCE": "PERSISTENCE",
    "DATA_GROUP": "DATA_GROUP",
    "FILE_SYSTEM": "FILE_SYSTEM",
    "NETWORK_SOCKET": "NETWORK_SOCKET",
    "UNKNOWN": "UNKNOWN",
}

# Layout resolver mapping: Python arg_role_resolver function names -> C++ enum
LAYOUT_RESOLVER_MAP: dict[str, str] = {}  # populated from known commands

# Commands that need specific layout resolvers (identified by name)
LAYOUT_RESOLVER_BY_NAME = {
    "if": "IF",
    "switch": "SWITCH",
    "try": "TRY",
    "foreach": "FOREACH",
    "lmap": "FOREACH",
    "when": "WHEN",
    "expect": "EXPECT",
    "expect_after": "EXPECT",
    "expect_before": "EXPECT",
    "expect_background": "EXPECT",
    "expect_tty": "EXPECT",
    "expect_user": "EXPECT",
    "tcltest::test": "TCLTEST",
    "oo::class": "OO_CLASS",
    "oo::define": "OO_DEFINE",
}

# Placeholders — populated lazily after CppArgPattern is defined.
ARG_PATTERN_OVERRIDES: dict[str, list] = {}
ARITY_OVERRIDES: dict[str, tuple[int, int, int, int]] = {
    "foreach": (3, 2147483647, 2, 1),  # min=3, odd argc
    "lmap": (3, 2147483647, 2, 1),
    "for": (4, 4, 0, 0),               # exactly 4 args
    "while": (2, 2, 0, 0),             # exactly 2 args
}

# Profile flag mapping
PROFILE_FLAG_MAP = {
    "HTTP": "HTTP",
    "FASTHTTP": "FASTHTTP",
    "SSL": "SSL",
    "SIP": "SIP",
    "DNS": "DNS",
    "FTP": "FTP",
    "DIAMETER": "DIAMETER",
    "RADIUS": "RADIUS",
    "STREAM": "STREAM",
    "WEBSOCKET": "WEBSOCKET",
    "MQTT": "MQTT",
}


# ─── Intermediate representations ──────────────────────────────────


@dataclass
class CppHoverEntry:
    index: int
    command_name: str
    subcmd_name: str | None
    summary: str
    synopsis: tuple[str, ...]
    snippet: str
    source: str
    examples: str
    return_value: str


@dataclass
class CppArgPattern:
    kind: str  # FIXED, TAIL, STRIDE, OPTION_VALUE
    role: str  # C++ enum name
    index: int = 0
    stride: int = 0
    option_name: str = ""


@dataclass
class CppOptionDesc:
    name: str
    takes_value: bool = False
    value_hint: str = ""
    is_terminator: bool = False
    detail: str = ""
    value_role: str = "VALUE"


@dataclass
class CppFormDesc:
    kind: str  # DEFAULT, GETTER, SETTER, ACTION
    arity_min: int = 0
    arity_max: int = 2147483647
    options: list[CppOptionDesc] = field(default_factory=list)
    pure: bool = False
    mutator: bool = False


@dataclass
class CppArgTypeDesc:
    index: int
    expected: str  # C++ TclType enum name
    shimmers: bool = False


@dataclass
class CppSideEffectDesc:
    target: str = "UNKNOWN"
    reads: bool = False
    writes: bool = False
    connection_side: str = "NONE"
    scope: str = "UNKNOWN"


@dataclass
class CppTaintHints:
    source_colour: str = "NONE"
    transform_colour: str = "NONE"
    safe_colour: str = "NONE"
    output_sink_code: str = ""


@dataclass
class CppSubCmdDesc:
    name: str
    arity_min: int = 0
    arity_max: int = 2147483647
    arity_modulus: int = 0
    arity_remainder: int = 0
    arg_patterns: list[CppArgPattern] = field(default_factory=list)
    options: list[CppOptionDesc] = field(default_factory=list)
    forms: list[CppFormDesc] = field(default_factory=list)
    arg_types: list[CppArgTypeDesc] = field(default_factory=list)
    dialects: str | None = None  # None = inherit, else "TCL84 | TCL85"
    pure: bool = False
    mutator: bool = False
    destructive: bool = False
    loop_list_header: bool = False
    creates_scope_alias: bool = False
    is_slot_command: bool = False
    defines_procedure: bool = False
    return_type: str = "STRING"
    pattern_type: str = "NONE"
    taint_hints: CppTaintHints = field(default_factory=CppTaintHints)
    credential_arg: int = -1
    sensitive_headers: list[str] = field(default_factory=list)
    deprecated_replacement: str = ""
    hover_index: int = -1
    format_string_type: str = "NONE"
    side_effect_hints: list[CppSideEffectDesc] = field(default_factory=list)


@dataclass
class CppCommandDesc:
    name: str
    arity_min: int = 0
    arity_max: int = 2147483647
    arity_modulus: int = 0
    arity_remainder: int = 0
    arg_patterns: list[CppArgPattern] = field(default_factory=list)
    layout_resolver: str = "NONE"
    subcommands: list[CppSubCmdDesc] = field(default_factory=list)
    allow_unknown_subcommands: bool = False
    options: list[CppOptionDesc] = field(default_factory=list)
    forms: list[CppFormDesc] = field(default_factory=list)
    dialects: str = "ALL"
    event_transport: str = "NONE"
    event_profiles: str = "NONE"
    pure: bool = False
    mutator: bool = False
    is_control_flow: bool = False
    never_inline_body: bool = False
    has_loop_body: bool = False
    loop_list_header: bool = False
    creates_dynamic_barrier: bool = False
    defines_procedure: bool = False
    is_scope_barrier: bool = False
    top_level_only: bool = False
    needs_start_cmd: bool = False
    warn_without_terminator: bool = False
    cse_candidate: bool = False
    compile_time_evaluable: bool = False
    unsafe: bool = False
    taint_sink: bool = False
    diagram_action: bool = False
    xc_translatable: int = 0
    assigns_variable_at: int = -1
    arg_types: list[CppArgTypeDesc] = field(default_factory=list)
    return_type: str = "STRING"
    side_effect_hints: list[CppSideEffectDesc] = field(default_factory=list)
    taint_hints: CppTaintHints = field(default_factory=CppTaintHints)
    required_package: str = ""
    tcllib_package: str = ""
    warn_missing_import: bool = True
    hover_index: int = -1
    deprecated_replacement: str = ""
    formatting_keywords: list[str] = field(default_factory=list)
    pattern_type: str = "NONE"
    format_string_type: str = "NONE"
    password_option_command: bool = False
    credential_options: list[str] = field(default_factory=list)
    sensitive_headers: list[str] = field(default_factory=list)


# ─── C++ arg pattern overrides ─────────────────────────────────────
# Commands where the C++ ArgPattern system can express patterns that the
# Python dict[int, ArgRole] cannot (stride, tail, modular arity, etc.).

def _init_arg_pattern_overrides():
    global ARG_PATTERN_OVERRIDES
    if ARG_PATTERN_OVERRIDES:
        return
    P = CppArgPattern
    ARG_PATTERN_OVERRIDES.update({
        "foreach": [
            P(kind="STRIDE", role="VALUE", index=0, stride=2),
            P(kind="STRIDE", role="VALUE", index=1, stride=2),
            P(kind="FIXED", role="BODY", index=-1),
        ],
        "lmap": [
            P(kind="STRIDE", role="VALUE", index=0, stride=2),
            P(kind="STRIDE", role="VALUE", index=1, stride=2),
            P(kind="FIXED", role="BODY", index=-1),
        ],
        "lassign": [
            P(kind="FIXED", role="VALUE", index=0),
            P(kind="TAIL", role="VAR_NAME", index=1),
        ],
        "eval": [
            P(kind="TAIL", role="BODY", index=0),
        ],
        "uplevel": [
            P(kind="TAIL", role="BODY", index=0),
        ],
        "regexp": [
            P(kind="FIXED", role="PATTERN", index=0),
            P(kind="FIXED", role="VALUE", index=1),
            P(kind="TAIL", role="VAR_NAME", index=2),
        ],
        "regsub": [
            P(kind="FIXED", role="PATTERN", index=0),
            P(kind="FIXED", role="VALUE", index=1),
            P(kind="FIXED", role="VALUE", index=2),
            P(kind="FIXED", role="VAR_NAME", index=3),
        ],
        "proc": [
            P(kind="FIXED", role="NAME", index=0),
            P(kind="FIXED", role="PARAM_LIST", index=1),
            P(kind="FIXED", role="BODY", index=2),
        ],
        "set": [
            P(kind="FIXED", role="VAR_NAME", index=0),
            P(kind="FIXED", role="VALUE", index=1),
        ],
        "foreach_in_collection": [
            P(kind="FIXED", role="VAR_NAME", index=0),
            P(kind="FIXED", role="VALUE", index=1),
            P(kind="FIXED", role="BODY", index=2),
        ],
        # FORMAT_STRING role (C++ only — Python has no FORMAT_STRING ArgRole)
        "format": [
            P(kind="FIXED", role="VALUE", index=0),  # format string (VALUE since FORMAT_STRING isn't in Python ArgRole)
            P(kind="TAIL", role="VALUE", index=1),
        ],
        "scan": [
            P(kind="FIXED", role="VALUE", index=0),
            P(kind="FIXED", role="VALUE", index=1),  # format string
            P(kind="TAIL", role="VAR_NAME", index=2),
        ],
        # Control flow
        "while": [
            P(kind="FIXED", role="EXPR", index=0),
            P(kind="FIXED", role="BODY", index=1),
        ],
        "for": [
            P(kind="FIXED", role="BODY", index=0),   # init
            P(kind="FIXED", role="EXPR", index=1),    # test
            P(kind="FIXED", role="BODY", index=2),    # next
            P(kind="FIXED", role="BODY", index=3),    # body
        ],
        "catch": [
            P(kind="FIXED", role="BODY", index=0),
            P(kind="FIXED", role="VAR_NAME", index=1),
            P(kind="FIXED", role="VAR_NAME", index=2),
        ],
        "apply": [
            P(kind="FIXED", role="BODY", index=0),
        ],
        "coroutine": [
            P(kind="FIXED", role="NAME", index=0),
            P(kind="FIXED", role="VALUE", index=1),   # command
        ],
        "tailcall": [
            P(kind="FIXED", role="VALUE", index=0),   # command name
        ],
        # Commands with CHANNEL role
        "puts": [
            P(kind="FIXED", role="VALUE", index=0),   # may be channel or string
        ],
        "gets": [
            P(kind="FIXED", role="CHANNEL", index=0),
            P(kind="FIXED", role="VAR_NAME", index=1),
        ],
        "read": [
            P(kind="FIXED", role="CHANNEL", index=0),
        ],
        "close": [
            P(kind="FIXED", role="CHANNEL", index=0),
        ],
        "eof": [
            P(kind="FIXED", role="CHANNEL", index=0),
        ],
        "flush": [
            P(kind="FIXED", role="CHANNEL", index=0),
        ],
        "tell": [
            P(kind="FIXED", role="CHANNEL", index=0),
        ],
        "seek": [
            P(kind="FIXED", role="CHANNEL", index=0),
            P(kind="FIXED", role="VALUE", index=1),
        ],
        "fileevent": [
            P(kind="FIXED", role="CHANNEL", index=0),
            P(kind="FIXED", role="VALUE", index=1),
            P(kind="FIXED", role="BODY", index=2),
        ],
        "fconfigure": [
            P(kind="FIXED", role="CHANNEL", index=0),
        ],
        # Variable commands
        "global": [
            P(kind="TAIL", role="VAR_NAME", index=0),
        ],
        "variable": [
            P(kind="TAIL", role="VAR_NAME", index=0),
        ],
        "upvar": [
            P(kind="TAIL", role="VAR_NAME", index=0),
        ],
        "unset": [
            P(kind="TAIL", role="VAR_NAME", index=0),
        ],
        "incr": [
            P(kind="FIXED", role="VAR_NAME", index=0),
            P(kind="FIXED", role="VALUE", index=1),
        ],
        # Pattern commands
        "lsearch": [
            P(kind="FIXED", role="VALUE", index=0),
            P(kind="FIXED", role="PATTERN", index=1),
        ],
        # Namespace
        "namespace": [],  # subcommand-based, handled by subcommands
        "source": [
            P(kind="FIXED", role="VALUE", index=0),
        ],
        "rename": [
            P(kind="FIXED", role="NAME", index=0),
            P(kind="FIXED", role="NAME", index=1),
        ],
        "return": [],  # option-heavy, no fixed pattern
    })


# ─── Conversion: Python CommandSpec -> CppCommandDesc ──────────────


def _convert_dialects(dialects: frozenset[str] | None) -> str:
    if dialects is None:
        return "ALL"
    flags = []
    for d in sorted(dialects):
        cpp_flag = DIALECT_FLAG_MAP.get(d)
        if cpp_flag:
            flags.append(cpp_flag)
    if not flags:
        return "ALL"
    return " | ".join(f"DialectFlags::{f}" for f in flags)


def _convert_arity(arity: Arity) -> tuple[int, int, int, int]:
    """Returns (min, max, modulus, remainder)."""
    mx = 2147483647 if arity.is_unlimited else arity.max
    return (arity.min, mx, 0, 0)


def _convert_arg_role(role: ArgRole) -> str:
    return ARGROLE_MAP.get(role, "VALUE")


def _convert_arg_patterns(arg_roles: dict[int, ArgRole]) -> list[CppArgPattern]:
    """Convert Python dict[int, ArgRole] to CppArgPattern list."""
    if not arg_roles:
        return []

    patterns = []
    # Detect TAIL patterns: consecutive indices from N to max with same role
    sorted_indices = sorted(arg_roles.keys())
    if sorted_indices:
        # Check for tail: if all indices from some point onward have the same role
        # and are consecutive
        tail_start = None
        tail_role = None
        for i in range(len(sorted_indices) - 1, -1, -1):
            idx = sorted_indices[i]
            role = arg_roles[idx]
            if tail_start is None:
                tail_start = idx
                tail_role = role
            elif idx == tail_start - 1 and role == tail_role:
                tail_start = idx
            else:
                break

        # If we have 3+ consecutive indices at the end with the same role, use TAIL
        if (
            tail_start is not None
            and tail_role is not None
            and sorted_indices[-1] - tail_start >= 2
        ):
            for idx in sorted_indices:
                if idx < tail_start:
                    patterns.append(CppArgPattern(
                        kind="FIXED",
                        role=_convert_arg_role(arg_roles[idx]),
                        index=idx,
                    ))
            patterns.append(CppArgPattern(
                kind="TAIL",
                role=_convert_arg_role(tail_role),
                index=tail_start,
            ))
        else:
            # All FIXED
            for idx in sorted_indices:
                patterns.append(CppArgPattern(
                    kind="FIXED",
                    role=_convert_arg_role(arg_roles[idx]),
                    index=idx,
                ))
    return patterns


def _convert_option(opt: OptionSpec) -> CppOptionDesc:
    return CppOptionDesc(
        name=opt.name,
        takes_value=opt.takes_value,
        value_hint=opt.value_hint or "",
        is_terminator=(opt.name == "--"),
        detail=opt.detail or "",
    )


def _convert_form(form: FormSpec) -> CppFormDesc:
    arity = form.arity
    if arity is None:
        arity_min, arity_max = 0, 2147483647
    else:
        arity_min = arity.min
        arity_max = 2147483647 if arity.is_unlimited else arity.max

    kind = FORMKIND_MAP.get(form.kind, "DEFAULT")
    options = [_convert_option(o) for o in form.options]

    return CppFormDesc(
        kind=kind,
        arity_min=arity_min,
        arity_max=arity_max,
        options=options,
        pure=form.pure,
        mutator=form.mutator,
    )


def _convert_tcl_type(tcl_type) -> str:
    """Convert Python TclType to C++ enum name."""
    if tcl_type is None:
        return "STRING"
    name = tcl_type.name if hasattr(tcl_type, "name") else str(tcl_type)
    return name.upper()


def _convert_side_effect(se) -> CppSideEffectDesc:
    """Convert a Python SideEffect to CppSideEffectDesc."""
    _ensure_side_effects()
    target = se.target.name if hasattr(se.target, "name") else "UNKNOWN"
    target = SIDE_EFFECT_TARGET_MAP.get(target, "UNKNOWN")
    cs = se.connection_side.name if hasattr(se.connection_side, "name") else "NONE"
    cs = CONNECTION_SIDE_MAP.get(cs, "NONE")
    scope = se.scope.name if hasattr(se, "scope") and se.scope is not None else "UNKNOWN"
    scope = STORAGE_SCOPE_MAP.get(scope, "UNKNOWN")
    return CppSideEffectDesc(
        target=target,
        reads=se.reads,
        writes=se.writes,
        connection_side=cs,
        scope=scope,
    )


def _convert_taint_hints_from_spec(spec: CommandSpec) -> CppTaintHints:
    """Extract basic taint hints from CommandSpec fields."""
    hints = CppTaintHints()
    if spec.taint_transform is not None:
        _ensure_taint()
        hints.transform_colour = "TAINTED"
    if spec.taint_output_sink:
        hints.output_sink_code = spec.taint_output_sink
    if spec.taint_sink_safe_colour is not None:
        hints.safe_colour = "TAINTED"
    return hints


def _convert_subcmd(sub: SubCommand, hover_entries: list[CppHoverEntry],
                    parent_name: str) -> CppSubCmdDesc:
    """Convert a Python SubCommand to CppSubCmdDesc."""
    arity_min, arity_max, _, _ = _convert_arity(sub.arity)
    arg_patterns = _convert_arg_patterns(sub.arg_roles)
    options = [_convert_option(o) for o in sub.options]

    forms = [_convert_form(f) for f in sub.forms]

    arg_types = []
    for idx, type_hint in sub.arg_types.items():
        arg_types.append(CppArgTypeDesc(
            index=idx,
            expected=_convert_tcl_type(type_hint.expected),
            shimmers=type_hint.shimmers if hasattr(type_hint, "shimmers") else False,
        ))

    # Hover — synthesize from detail/synopsis if no full hover object
    hover_index = -1
    if sub.hover:
        hover_index = len(hover_entries)
        hover_entries.append(CppHoverEntry(
            index=hover_index,
            command_name=parent_name,
            subcmd_name=sub.name,
            summary=sub.hover.summary,
            synopsis=sub.hover.synopsis,
            snippet=sub.hover.snippet,
            source=sub.hover.source,
            examples=sub.hover.examples,
            return_value=sub.hover.return_value,
        ))
    elif sub.detail or sub.synopsis:
        hover_index = len(hover_entries)
        synopsis = (sub.synopsis,) if sub.synopsis else ()
        hover_entries.append(CppHoverEntry(
            index=hover_index,
            command_name=parent_name,
            subcmd_name=sub.name,
            summary=sub.detail or "",
            synopsis=synopsis,
            snippet="",
            source="",
            examples="",
            return_value="",
        ))

    # Dialect
    dialects = None
    if sub.dialects is not None:
        dialects = _convert_dialects(sub.dialects)

    # Side effects
    side_effects = []
    if sub.side_effect_hints:
        for se in sub.side_effect_hints:
            side_effects.append(_convert_side_effect(se))

    # Taint
    taint = CppTaintHints()
    if sub.taint_transform is not None:
        taint.transform_colour = "TAINTED"
    if sub.taint_output_sink:
        taint.output_sink_code = sub.taint_output_sink

    return CppSubCmdDesc(
        name=sub.name,
        arity_min=arity_min,
        arity_max=arity_max,
        arg_patterns=arg_patterns,
        options=options,
        forms=forms,
        arg_types=arg_types,
        dialects=dialects,
        pure=sub.pure,
        mutator=sub.mutator,
        destructive=sub.destructive,
        loop_list_header=sub.loop_list_header,
        creates_scope_alias=sub.creates_scope_alias,
        defines_procedure=getattr(sub, "defines_procedure", False),
        return_type=_convert_tcl_type(sub.return_type),
        pattern_type=PATTERNTYPE_MAP.get(
            sub.pattern_type.value if sub.pattern_type else "", "NONE"
        ),
        taint_hints=taint,
        credential_arg=sub.credential_arg if sub.credential_arg is not None else -1,
        sensitive_headers=list(sub.sensitive_headers) if sub.sensitive_headers else [],
        deprecated_replacement=(
            sub.deprecated_replacement_name or ""
        ),
        hover_index=hover_index,
        format_string_type=FORMATTYPE_MAP.get(
            sub.format_string_type.value if sub.format_string_type else "", "NONE"
        ),
        side_effect_hints=side_effects,
    )


def _merge_enrichment(
    hover: CppHoverEntry,
    enrichment: dict[str, str],
) -> None:
    """Merge enrichment data into a hover entry (fills gaps only)."""
    if not hover.snippet and enrichment.get("snippet"):
        hover.snippet = enrichment["snippet"]
    if not hover.examples and enrichment.get("examples"):
        hover.examples = enrichment["examples"]
    if not hover.return_value and enrichment.get("return_value"):
        hover.return_value = enrichment["return_value"]
    if not hover.summary and enrichment.get("summary"):
        hover.summary = enrichment["summary"]
    if not hover.synopsis and enrichment.get("synopsis"):
        syn = enrichment["synopsis"]
        if isinstance(syn, (list, tuple)):
            hover.synopsis = tuple(syn)
        elif isinstance(syn, str) and syn:
            hover.synopsis = (syn,)
    if not hover.source and enrichment.get("source"):
        hover.source = enrichment["source"]


def convert_spec(
    spec: CommandSpec,
    hover_entries: list[CppHoverEntry],
    enrichment: dict[str, dict[str, str]] | None = None,
) -> CppCommandDesc:
    """Convert a Python CommandSpec to a CppCommandDesc."""
    _init_arg_pattern_overrides()
    arity_min, arity_max, modulus, remainder = (0, 2147483647, 0, 0)
    if spec.validation:
        arity_min, arity_max, modulus, remainder = _convert_arity(spec.validation.arity)

    # Apply arity overrides for commands with modular constraints
    if spec.name in ARITY_OVERRIDES:
        arity_min, arity_max, modulus, remainder = ARITY_OVERRIDES[spec.name]

    # Arg patterns — use C++ overrides if available, otherwise auto-convert
    if spec.name in ARG_PATTERN_OVERRIDES:
        arg_patterns = ARG_PATTERN_OVERRIDES[spec.name]
    else:
        arg_patterns = _convert_arg_patterns(spec.arg_roles)

    # Layout resolver
    layout = LAYOUT_RESOLVER_BY_NAME.get(spec.name, "NONE")
    if layout == "NONE" and spec.arg_role_resolver is not None:
        # Try to match by function name
        fn_name = getattr(spec.arg_role_resolver, "__name__", "")
        layout = LAYOUT_RESOLVER_MAP.get(fn_name, "NONE")

    # Options (from forms)
    all_options: list[CppOptionDesc] = []
    seen_opts: set[str] = set()
    for form in spec.forms:
        for opt in form.options:
            if opt.name not in seen_opts:
                seen_opts.add(opt.name)
                all_options.append(_convert_option(opt))

    # Forms
    forms = [_convert_form(f) for f in spec.forms]

    # Subcommands
    subcmds = []
    for sub_name, sub in spec.subcommands.items():
        subcmds.append(_convert_subcmd(sub, hover_entries, spec.name))

    # Dialects
    dialects = _convert_dialects(spec.dialects)

    # Event requires
    event_transport = "NONE"
    event_profiles = "NONE"
    if spec.event_requires is not None:
        er = spec.event_requires
        transport = getattr(er, "transport", None)
        if transport:
            event_transport = transport.upper() if isinstance(transport, str) else "NONE"
            if event_transport not in ("TCP", "UDP", "ANY"):
                event_transport = "NONE"
        profiles = getattr(er, "profiles", None)
        if profiles:
            pflags = []
            for p in sorted(profiles):
                cpp_p = PROFILE_FLAG_MAP.get(p.upper(), PROFILE_FLAG_MAP.get(p))
                if cpp_p:
                    pflags.append(f"ProfileFlags::{cpp_p}")
            if pflags:
                event_profiles = " | ".join(pflags)

    # Arg types
    arg_types = []
    for idx, type_hint in spec.arg_types.items():
        arg_types.append(CppArgTypeDesc(
            index=idx,
            expected=_convert_tcl_type(type_hint.expected),
            shimmers=type_hint.shimmers if hasattr(type_hint, "shimmers") else False,
        ))

    # Side effects
    side_effects = []
    if spec.side_effect_hints:
        for se in spec.side_effect_hints:
            side_effects.append(_convert_side_effect(se))

    # Taint
    taint = _convert_taint_hints_from_spec(spec)

    # Hover
    hover_index = -1
    if spec.hover:
        hover_index = len(hover_entries)
        hover_entries.append(CppHoverEntry(
            index=hover_index,
            command_name=spec.name,
            subcmd_name=None,
            summary=spec.hover.summary,
            synopsis=spec.hover.synopsis,
            snippet=spec.hover.snippet,
            source=spec.hover.source,
            examples=spec.hover.examples,
            return_value=spec.hover.return_value,
        ))

    # Apply enrichment to the hover entry
    if enrichment and spec.name in enrichment and hover_index >= 0:
        _merge_enrichment(hover_entries[hover_index], enrichment[spec.name])

    # Formatting keywords (for variable-layout commands)
    formatting_keywords: list[str] = []
    if spec.name == "if":
        formatting_keywords = ["then", "else", "elseif"]
    elif spec.name == "switch":
        formatting_keywords = []
    elif spec.name == "try":
        formatting_keywords = ["on", "trap", "finally"]

    return CppCommandDesc(
        name=spec.name,
        arity_min=arity_min,
        arity_max=arity_max,
        arity_modulus=modulus,
        arity_remainder=remainder,
        arg_patterns=arg_patterns,
        layout_resolver=layout,
        subcommands=subcmds,
        allow_unknown_subcommands=spec.allow_unknown_subcommands,
        options=all_options,
        forms=forms,
        dialects=dialects,
        event_transport=event_transport,
        event_profiles=event_profiles,
        pure=spec.pure,
        mutator=getattr(spec, "mutator", False),
        is_control_flow=spec.is_control_flow,
        never_inline_body=spec.never_inline_body,
        has_loop_body=spec.has_loop_body,
        loop_list_header=spec.loop_list_header,
        creates_dynamic_barrier=spec.creates_dynamic_barrier,
        defines_procedure=spec.defines_procedure,
        is_scope_barrier=spec.name in ("proc", "apply", "method", "oo::define"),
        top_level_only=spec.name in ("when",),
        needs_start_cmd=spec.needs_start_cmd,
        warn_without_terminator=spec.warn_without_terminator,
        cse_candidate=spec.cse_candidate,
        unsafe=spec.unsafe,
        taint_sink=spec.taint_sink,
        diagram_action=spec.diagram_action,
        xc_translatable=(
            1 if spec.xc_translatable is True
            else -1 if spec.xc_translatable is False
            else 0
        ),
        assigns_variable_at=(
            spec.assigns_variable_at if spec.assigns_variable_at is not None else -1
        ),
        arg_types=arg_types,
        return_type=_convert_tcl_type(spec.return_type),
        side_effect_hints=side_effects,
        taint_hints=taint,
        required_package=spec.required_package or "",
        tcllib_package=spec.tcllib_package or "",
        warn_missing_import=spec.warn_missing_import,
        hover_index=hover_index,
        deprecated_replacement=(
            spec.deprecated_replacement_name or ""
        ),
        formatting_keywords=formatting_keywords,
        pattern_type=PATTERNTYPE_MAP.get(
            spec.pattern_type.value if spec.pattern_type else "", "NONE"
        ),
        format_string_type=FORMATTYPE_MAP.get(
            spec.format_string_type.value if spec.format_string_type else "", "NONE"
        ),
        password_option_command=spec.password_option_command,
        credential_options=list(spec.credential_options) if spec.credential_options else [],
        sensitive_headers=list(spec.sensitive_headers) if spec.sensitive_headers else [],
    )


# ─── C++ code emitter ──────────────────────────────────────────────


def _emit_arg_patterns(patterns: list[CppArgPattern], prefix: str) -> str:
    if not patterns:
        return ""
    lines = [f"inline constexpr ArgPattern {prefix}_args[] = {{"]
    for p in patterns:
        parts = [f".kind = {p.kind}", f".role = {p.role}", f".index = {p.index}"]
        if p.stride:
            parts.append(f".stride = {p.stride}")
        if p.option_name:
            parts.append(f'.option_name = "{_c_string_escape(p.option_name)}"')
        lines.append(f"    {{{', '.join(parts)}}},")
    lines.append("};")
    return "\n".join(lines)


def _emit_options(options: list[CppOptionDesc], prefix: str) -> str:
    if not options:
        return ""
    lines = [f"inline constexpr OptionDesc {prefix}_opts[] = {{"]
    for o in options:
        parts = [f'.name = "{_c_string_escape(o.name)}"']
        if o.takes_value:
            parts.append(".takes_value = true")
        if o.value_hint:
            parts.append(f'.value_hint = "{_c_string_escape(o.value_hint)}"')
        if o.is_terminator:
            parts.append(".is_terminator = true")
        if o.detail:
            parts.append(f'.detail = "{_c_string_escape(o.detail)}"')
        if o.value_role != "VALUE":
            parts.append(f".value_role = {o.value_role}")
        lines.append(f"    {{{', '.join(parts)}}},")
    lines.append("};")
    return "\n".join(lines)


def _emit_arg_types(arg_types: list[CppArgTypeDesc], prefix: str) -> str:
    if not arg_types:
        return ""
    lines = [f"inline constexpr ArgTypeDesc {prefix}_types[] = {{"]
    for at in arg_types:
        parts = [f".index = {at.index}", f".expected = TclType::{at.expected}"]
        if at.shimmers:
            parts.append(".shimmers = true")
        lines.append(f"    {{{', '.join(parts)}}},")
    lines.append("};")
    return "\n".join(lines)


def _emit_forms(forms: list[CppFormDesc], prefix: str) -> str:
    if not forms:
        return ""
    lines = [f"inline constexpr FormDesc {prefix}_forms[] = {{"]
    for f in forms:
        parts = [f".kind = FormKind::{f.kind}"]
        arity_max_str = "std::numeric_limits<std::int32_t>::max()" if f.arity_max >= 2147483647 else str(f.arity_max)
        parts.append(f".arity = {{{f.arity_min}, {arity_max_str}}}")
        if f.options:
            # Forms with their own options need separate arrays - skip for now,
            # use command-level options
            pass
        if f.pure:
            parts.append(".pure = true")
        if f.mutator:
            parts.append(".mutator = true")
        lines.append(f"    {{{', '.join(parts)}}},")
    lines.append("};")
    return "\n".join(lines)


def _emit_side_effects(effects: list[CppSideEffectDesc], prefix: str) -> str:
    if not effects:
        return ""
    lines = [f"inline constexpr SideEffectDesc {prefix}_effects[] = {{"]
    for e in effects:
        parts = [f".target = SideEffectTarget::{e.target}"]
        if e.reads:
            parts.append(".reads = true")
        if e.writes:
            parts.append(".writes = true")
        if e.connection_side != "NONE":
            parts.append(f".connection_side = ConnectionSide::{e.connection_side}")
        if e.scope != "UNKNOWN":
            parts.append(f".scope = StorageScope::{e.scope}")
        lines.append(f"    {{{', '.join(parts)}}},")
    lines.append("};")
    return "\n".join(lines)


def _emit_formatting_keywords(keywords: list[str], prefix: str) -> str:
    if not keywords:
        return ""
    lines = [f"inline constexpr std::string_view {prefix}_keywords[] = {{"]
    for kw in keywords:
        lines.append(f'    "{_c_string_escape(kw)}",')
    lines.append("};")
    return "\n".join(lines)


def _emit_string_array(strings: list[str], prefix: str, suffix: str) -> str:
    if not strings:
        return ""
    lines = [f"inline constexpr std::string_view {prefix}_{suffix}[] = {{"]
    for s in strings:
        lines.append(f'    "{_c_string_escape(s)}",')
    lines.append("};")
    return "\n".join(lines)


def _emit_subcmd_options(subcmds: list[CppSubCmdDesc], prefix: str) -> str:
    """Emit option arrays for subcommands that have them."""
    parts = []
    for sub in subcmds:
        if sub.options:
            sub_prefix = f"{prefix}_sub_{_safe_ident(sub.name)}"
            parts.append(_emit_options(sub.options, sub_prefix))
    return "\n\n".join(parts)


def _emit_subcmd_arg_patterns(subcmds: list[CppSubCmdDesc], prefix: str) -> str:
    """Emit arg pattern arrays for subcommands that have them."""
    parts = []
    for sub in subcmds:
        if sub.arg_patterns:
            sub_prefix = f"{prefix}_sub_{_safe_ident(sub.name)}"
            parts.append(_emit_arg_patterns(sub.arg_patterns, sub_prefix))
    return "\n\n".join(parts)


def _emit_subcmd_arg_types(subcmds: list[CppSubCmdDesc], prefix: str) -> str:
    """Emit arg type arrays for subcommands that have them."""
    parts = []
    for sub in subcmds:
        if sub.arg_types:
            sub_prefix = f"{prefix}_sub_{_safe_ident(sub.name)}"
            parts.append(_emit_arg_types(sub.arg_types, sub_prefix))
    return "\n\n".join(parts)


def _emit_subcmd_forms(subcmds: list[CppSubCmdDesc], prefix: str) -> str:
    """Emit form arrays for subcommands that have them."""
    parts = []
    for sub in subcmds:
        if sub.forms:
            sub_prefix = f"{prefix}_sub_{_safe_ident(sub.name)}"
            parts.append(_emit_forms(sub.forms, sub_prefix))
    return "\n\n".join(parts)


def _emit_subcmds(subcmds: list[CppSubCmdDesc], prefix: str) -> str:
    if not subcmds:
        return ""
    lines = [f"inline constexpr SubCmdDesc {prefix}_subcmds[] = {{"]
    for sub in subcmds:
        sub_ident = _safe_ident(sub.name)
        sub_prefix = f"{prefix}_sub_{sub_ident}"
        parts = [f'.name = "{_c_string_escape(sub.name)}"']

        arity_max_str = (
            "std::numeric_limits<std::int32_t>::max()"
            if sub.arity_max >= 2147483647
            else str(sub.arity_max)
        )
        if sub.arity_modulus:
            parts.append(
                f".arity = {{{sub.arity_min}, {arity_max_str}, "
                f"{sub.arity_modulus}, {sub.arity_remainder}}}"
            )
        else:
            parts.append(f".arity = {{{sub.arity_min}, {arity_max_str}}}")

        if sub.arg_patterns:
            parts.append(f".arg_patterns = {sub_prefix}_args")
        if sub.options:
            parts.append(f".options = {sub_prefix}_opts")
        if sub.forms:
            parts.append(f".forms = {sub_prefix}_forms")
        if sub.arg_types:
            parts.append(f".arg_types = {sub_prefix}_types")
        if sub.dialects is not None:
            parts.append(f".dialects = {sub.dialects}")
        if sub.pure:
            parts.append(".pure = true")
        if sub.mutator:
            parts.append(".mutator = true")
        if sub.destructive:
            parts.append(".destructive = true")
        if sub.loop_list_header:
            parts.append(".loop_list_header = true")
        if sub.creates_scope_alias:
            parts.append(".creates_scope_alias = true")
        if sub.is_slot_command:
            parts.append(".is_slot_command = true")
        if sub.defines_procedure:
            parts.append(".defines_procedure = true")
        if sub.return_type != "STRING":
            parts.append(f".return_type = TclType::{sub.return_type}")
        if sub.pattern_type != "NONE":
            parts.append(f".pattern_type = PatternType::{sub.pattern_type}")
        if (
            sub.taint_hints.transform_colour != "NONE"
            or sub.taint_hints.output_sink_code
            or sub.taint_hints.safe_colour != "NONE"
        ):
            tparts = []
            if sub.taint_hints.transform_colour != "NONE":
                tparts.append(
                    f".transform_colour = TaintColour::{sub.taint_hints.transform_colour}"
                )
            if sub.taint_hints.output_sink_code:
                tparts.append(
                    f'.output_sink_code = "{_c_string_escape(sub.taint_hints.output_sink_code)}"'
                )
            if sub.taint_hints.safe_colour != "NONE":
                tparts.append(
                    f".safe_colour = TaintColour::{sub.taint_hints.safe_colour}"
                )
            parts.append(f".taint_hints = {{{', '.join(tparts)}}}")
        if sub.credential_arg >= 0:
            parts.append(f".credential_arg = {sub.credential_arg}")
        if sub.deprecated_replacement:
            parts.append(
                f'.deprecated_replacement = "{_c_string_escape(sub.deprecated_replacement)}"'
            )
        if sub.hover_index >= 0:
            parts.append(f".hover_index = {sub.hover_index}")

        lines.append(f"    {{{', '.join(parts)}}},")
    lines.append("};")
    return "\n".join(lines)


def emit_command(cmd: CppCommandDesc, category: str = "") -> str:
    """Emit all C++ code for a single command descriptor."""
    prefix = f"{category}_{_safe_ident(cmd.name)}" if category else _safe_ident(cmd.name)
    sections: list[str] = []

    # Supporting arrays
    if cmd.arg_patterns:
        sections.append(_emit_arg_patterns(cmd.arg_patterns, prefix))
    if cmd.options:
        sections.append(_emit_options(cmd.options, prefix))
    if cmd.arg_types:
        sections.append(_emit_arg_types(cmd.arg_types, prefix))
    if cmd.forms:
        sections.append(_emit_forms(cmd.forms, prefix))
    if cmd.side_effect_hints:
        sections.append(_emit_side_effects(cmd.side_effect_hints, prefix))
    if cmd.formatting_keywords:
        sections.append(_emit_formatting_keywords(cmd.formatting_keywords, prefix))
    if cmd.credential_options:
        sections.append(_emit_string_array(cmd.credential_options, prefix, "cred_opts"))
    if cmd.sensitive_headers:
        sections.append(_emit_string_array(cmd.sensitive_headers, prefix, "sens_hdrs"))

    # Subcommand supporting arrays
    if cmd.subcommands:
        sub_opts = _emit_subcmd_options(cmd.subcommands, prefix)
        if sub_opts:
            sections.append(sub_opts)
        sub_args = _emit_subcmd_arg_patterns(cmd.subcommands, prefix)
        if sub_args:
            sections.append(sub_args)
        sub_types = _emit_subcmd_arg_types(cmd.subcommands, prefix)
        if sub_types:
            sections.append(sub_types)
        sub_forms = _emit_subcmd_forms(cmd.subcommands, prefix)
        if sub_forms:
            sections.append(sub_forms)
        sections.append(_emit_subcmds(cmd.subcommands, prefix))

    # Main CommandDesc
    parts = [f'.name = "{_c_string_escape(cmd.name)}"']

    # Arity
    arity_max_str = (
        "std::numeric_limits<std::int32_t>::max()"
        if cmd.arity_max >= 2147483647
        else str(cmd.arity_max)
    )
    if cmd.arity_modulus:
        parts.append(
            f".arity = {{{cmd.arity_min}, {arity_max_str}, "
            f"{cmd.arity_modulus}, {cmd.arity_remainder}}}"
        )
    elif cmd.arity_min > 0 or cmd.arity_max < 2147483647:
        parts.append(f".arity = {{{cmd.arity_min}, {arity_max_str}}}")

    if cmd.arg_patterns:
        parts.append(f".arg_patterns = {prefix}_args")
    if cmd.layout_resolver != "NONE":
        parts.append(f".layout_resolver = LayoutResolver::{cmd.layout_resolver}")
    if cmd.subcommands:
        parts.append(f".subcommands = {prefix}_subcmds")
    if cmd.allow_unknown_subcommands:
        parts.append(".allow_unknown_subcommands = true")
    if cmd.options:
        parts.append(f".options = {prefix}_opts")
    if cmd.forms:
        parts.append(f".forms = {prefix}_forms")
    if cmd.dialects != "ALL":
        parts.append(f".dialects = {cmd.dialects}")
    if cmd.event_transport != "NONE" or cmd.event_profiles != "NONE":
        er_parts = []
        if cmd.event_transport != "NONE":
            er_parts.append(f".transport = Transport::{cmd.event_transport}")
        if cmd.event_profiles != "NONE":
            er_parts.append(f".profiles = {cmd.event_profiles}")
        parts.append(f".event_requires = {{{', '.join(er_parts)}}}")

    # Boolean flags
    for flag_name, flag_val in [
        ("pure", cmd.pure),
        ("mutator", cmd.mutator),
        ("is_control_flow", cmd.is_control_flow),
        ("never_inline_body", cmd.never_inline_body),
        ("has_loop_body", cmd.has_loop_body),
        ("loop_list_header", cmd.loop_list_header),
        ("creates_dynamic_barrier", cmd.creates_dynamic_barrier),
        ("defines_procedure", cmd.defines_procedure),
        ("is_scope_barrier", cmd.is_scope_barrier),
        ("top_level_only", cmd.top_level_only),
        ("needs_start_cmd", cmd.needs_start_cmd),
        ("warn_without_terminator", cmd.warn_without_terminator),
        ("cse_candidate", cmd.cse_candidate),
        ("compile_time_evaluable", cmd.compile_time_evaluable),
        ("unsafe", cmd.unsafe),
        ("taint_sink", cmd.taint_sink),
        ("diagram_action", cmd.diagram_action),
    ]:
        if flag_val:
            parts.append(f".{flag_name} = true")

    if cmd.xc_translatable != 0:
        parts.append(f".xc_translatable = {cmd.xc_translatable}")
    if cmd.assigns_variable_at >= 0:
        parts.append(f".assigns_variable_at = {cmd.assigns_variable_at}")
    if cmd.arg_types:
        parts.append(f".arg_types = {prefix}_types")
    if cmd.return_type != "STRING":
        parts.append(f".return_type = TclType::{cmd.return_type}")
    if cmd.side_effect_hints:
        parts.append(f".side_effect_hints = {prefix}_effects")

    # Taint hints
    if (
        cmd.taint_hints.transform_colour != "NONE"
        or cmd.taint_hints.output_sink_code
        or cmd.taint_hints.safe_colour != "NONE"
        or cmd.taint_hints.source_colour != "NONE"
    ):
        tparts = []
        if cmd.taint_hints.source_colour != "NONE":
            tparts.append(f".source_colour = TaintColour::{cmd.taint_hints.source_colour}")
        if cmd.taint_hints.transform_colour != "NONE":
            tparts.append(f".transform_colour = TaintColour::{cmd.taint_hints.transform_colour}")
        if cmd.taint_hints.safe_colour != "NONE":
            tparts.append(f".safe_colour = TaintColour::{cmd.taint_hints.safe_colour}")
        if cmd.taint_hints.output_sink_code:
            tparts.append(
                f'.output_sink_code = "{_c_string_escape(cmd.taint_hints.output_sink_code)}"'
            )
        parts.append(f".taint_hints = {{{', '.join(tparts)}}}")

    if cmd.required_package:
        parts.append(f'.required_package = "{_c_string_escape(cmd.required_package)}"')
    if cmd.tcllib_package:
        parts.append(f'.tcllib_package = "{_c_string_escape(cmd.tcllib_package)}"')
    if not cmd.warn_missing_import:
        parts.append(".warn_missing_import = false")
    if cmd.hover_index >= 0:
        parts.append(f".hover_index = {cmd.hover_index}")
    if cmd.deprecated_replacement:
        parts.append(
            f'.deprecated_replacement = "{_c_string_escape(cmd.deprecated_replacement)}"'
        )
    if cmd.formatting_keywords:
        parts.append(f".formatting_keywords = {prefix}_keywords")
    if cmd.pattern_type != "NONE":
        parts.append(f".pattern_type = PatternType::{cmd.pattern_type}")
    if cmd.format_string_type != "NONE":
        parts.append(f".format_string_type = FormatType::{cmd.format_string_type}")
    if cmd.password_option_command:
        parts.append(".password_option_command = true")
    if cmd.credential_options:
        parts.append(f".credential_options = {prefix}_cred_opts")
    if cmd.sensitive_headers:
        parts.append(f".sensitive_headers = {prefix}_sens_hdrs")

    # Format with line breaks for readability
    cmd_lines = [f"inline constexpr CommandDesc {prefix}_cmd{{"]
    for p in parts:
        cmd_lines.append(f"    {p},")
    cmd_lines.append("};")

    sections.append("\n".join(cmd_lines))
    return "\n\n".join(s for s in sections if s)


def emit_hover_table(entries: list[CppHoverEntry]) -> str:
    """Emit the hover table and synopsis backing arrays."""
    sections: list[str] = []

    # Synopsis backing arrays — use hover index for uniqueness
    for entry in entries:
        if entry.synopsis:
            lines = [f"inline constexpr std::string_view synopsis_{entry.index}[] = {{"]
            for syn in entry.synopsis:
                lines.append(f'    "{_c_string_escape(syn)}",')
            lines.append("};")
            sections.append("\n".join(lines))

    # Hover table
    lines = ["inline constexpr HoverText hover_table[] = {"]
    for entry in entries:
        parts = []
        if entry.summary:
            parts.append(f'.summary = "{_c_string_escape(entry.summary)}"')
        if entry.synopsis:
            parts.append(f".synopsis = synopsis_{entry.index}")
        if entry.snippet:
            parts.append(f'.snippet = "{_c_string_escape(entry.snippet)}"')
        if entry.source:
            parts.append(f'.source = "{_c_string_escape(entry.source)}"')
        if entry.examples:
            parts.append(f'.examples = "{_c_string_escape(entry.examples)}"')
        if entry.return_value:
            parts.append(f'.return_value = "{_c_string_escape(entry.return_value)}"')

        comment = entry.command_name
        if entry.subcmd_name:
            comment += f" {entry.subcmd_name}"
        lines.append(f"    // [{entry.index}] {comment}")
        lines.append(f"    {{{', '.join(parts)}}},")
    lines.append("};")
    sections.append("\n".join(lines))

    return "\n\n".join(sections)


# ─── File generation ───────────────────────────────────────────────

FILE_HEADER = """\
// AUTO-GENERATED by scripts/registry/generate_native_registry.py
// DO NOT EDIT — re-run the generator to update.

#include "tcl_lsp/registry/command_desc.hpp"

#include <limits>

namespace tcl_lsp::generated {

using enum ArgPatternKind;
using enum ArgRole;

"""

FILE_FOOTER = """
} // namespace tcl_lsp::generated
"""


def generate_category_inc(
    category: str,
    specs: list[CommandSpec],
    hover_entries: list[CppHoverEntry],
    enrichment: dict[str, dict[str, str]] | None = None,
) -> tuple[str, list[tuple[str, CppCommandDesc]]]:
    """Generate a .inc file for a category. Returns (content, [(prefix, cmd)])."""
    commands: list[tuple[str, CppCommandDesc]] = []

    content_parts = [f"// ── {category} commands ──\n"]

    for spec in sorted(specs, key=lambda s: s.name):
        cmd = convert_spec(spec, hover_entries, enrichment)
        prefix = f"{category}_{_safe_ident(cmd.name)}"
        commands.append((prefix, cmd))
        content_parts.append(f"// {cmd.name}")
        content_parts.append(emit_command(cmd, category))
        content_parts.append("")

    return "\n".join(content_parts), commands


def generate_all_commands_cpp(
    all_commands: list[tuple[str, CppCommandDesc]],
    hover_entries: list[CppHoverEntry],
    category_files: list[str],
) -> str:
    """Generate the master all_commands.cpp file."""
    lines = [FILE_HEADER.rstrip()]

    # Include category .inc files
    for f in category_files:
        lines.append(f'#include "{f}"')
    lines.append("")

    # Hover table
    lines.append(emit_hover_table(hover_entries))
    lines.append("")

    # Master command pointer table
    lines.append("inline constexpr const CommandDesc* all_commands[] = {")
    for prefix, cmd in sorted(all_commands, key=lambda c: c[1].name):
        lines.append(f"    &{prefix}_cmd,")
    lines.append("};")

    lines.append(FILE_FOOTER)
    return "\n".join(lines)


# ─── Hover enrichment from reference materials ─────────────────────


def _load_tcl_manpage_enrichment(doc_dir: Path) -> dict[str, dict[str, str]]:
    """Parse Tcl man pages and return {command_name: {summary, snippet, synopsis...}}.

    Tcl/Tk/Expect man pages use BSD-style licenses — verbatim text is permitted.
    """
    sys.path.insert(0, str(ROOT / "scripts" / "registry"))
    try:
        from scaffold_tcl_commands import (
            _parse_page, _select_synopsis, _build_page_index,
            _extract_section, _clean_nroff,
        )
    except ImportError:
        return {}

    if not doc_dir.exists():
        return {}

    pages, command_to_page = _build_page_index(doc_dir)
    enrichment: dict[str, dict[str, str]] = {}

    def _extract_full_section(lines: list[str], title: str, max_chars: int = 600) -> str:
        """Extract a full section as cleaned text, truncated to max_chars."""
        section = _extract_section(lines, title)
        if not section:
            return ""
        paragraphs: list[str] = []
        current: list[str] = []
        for line in section:
            if line.startswith(".SH "):
                break
            if line.startswith("'"):
                continue
            if line.startswith((".PP", ".P", ".LP")):
                if current:
                    paragraphs.append(" ".join(current))
                    current = []
                continue
            if line.startswith((".nf", ".fi", ".RS", ".RE", ".TP", ".IP", ".VS", ".VE")):
                continue
            if line.startswith("."):
                continue
            cleaned = _clean_nroff(line)
            if cleaned:
                current.append(cleaned)
        if current:
            paragraphs.append(" ".join(current))

        text = "\n\n".join(paragraphs)
        if len(text) > max_chars:
            text = text[:max_chars - 1].rstrip() + "\u2026"
        return text

    def _extract_examples(lines: list[str]) -> str:
        """Extract the EXAMPLES section as cleaned code."""
        section = _extract_section(lines, "EXAMPLES")
        if not section:
            return ""
        code_lines: list[str] = []
        in_code = False
        for line in section:
            if line.startswith(".SH "):
                break
            if line.startswith((".CS", ".nf")):
                in_code = True
                continue
            if line.startswith((".CE", ".fi")):
                in_code = False
                continue
            if line.startswith("'"):
                continue
            # Skip nroff directives
            if line.startswith((".ta", ".PP", ".P", ".LP", ".RS", ".RE",
                                ".TP", ".IP", ".VS", ".VE", ".sp", ".br")):
                continue
            if in_code:
                cleaned = _clean_nroff(line)
                code_lines.append(cleaned)
            elif not line.startswith("."):
                cleaned = _clean_nroff(line)
                if cleaned:
                    code_lines.append(cleaned)

        text = "\n".join(code_lines).strip()
        # Replace nroff tab escapes
        text = text.replace("\\et", "\t").replace("\\t", "\t")
        # Take just the first example block if there are multiple
        if "\n\n\n" in text:
            text = text.split("\n\n\n")[0].strip()
        # Truncate long examples
        if len(text) > 400:
            lines_list = text.split("\n")
            text = "\n".join(lines_list[:15])
        return text

    for cmd_name, page_file in command_to_page.items():
        page = pages.get(page_file, {})
        if not page:
            continue

        # Re-read raw lines for full section extraction
        page_path = doc_dir / page_file
        if not page_path.exists():
            continue
        raw_lines = page_path.read_text(encoding="utf-8", errors="ignore").splitlines()

        synopsis = _select_synopsis(cmd_name, page)
        summary = (
            page.get("summary_by_name", {}).get(cmd_name, "")
            or next(iter(page.get("summary_by_name", {}).values()), "")
        )
        # Use full description, not just first sentence
        snippet = _extract_full_section(raw_lines, "DESCRIPTION", max_chars=500)
        examples = _extract_examples(raw_lines)

        enrichment[cmd_name] = {
            "summary": summary,
            "synopsis": tuple(synopsis),
            "snippet": snippet,
            "examples": examples,
            "source": f"Tcl {page_file}",
        }

    return enrichment


def _load_irule_manindex_enrichment(man_dir: Path) -> dict[str, dict[str, str]]:
    """Parse the ltm_irule_man_index file from a BigIP extract.

    This file has structured entries with SYNOPSIS, DESCRIPTION, RETURN VALUE,
    VALID DURING, and EXAMPLES sections. We extract RETURN VALUE (factual data)
    and VALID DURING (event lists) which are not copyrightable.
    """
    man_index = man_dir / "ltm_irule_man_index"
    if not man_index.exists():
        return {}

    text = man_index.read_text(encoding="latin-1")
    entries: dict[str, dict[str, str]] = {}

    # Split into entries by "f5_help_search ltm_rule_command_" or "f5_help_search ltm_rule_event_"
    import re
    entry_pattern = re.compile(r"^f5_help_search ltm_rule_(?:command|event)_(.+)$", re.MULTILINE)
    splits = list(entry_pattern.finditer(text))

    for i, match in enumerate(splits):
        name = match.group(1)
        start = match.end()
        end = splits[i + 1].start() if i + 1 < len(splits) else len(text)
        block = text[start:end].strip()

        entry: dict[str, str] = {}

        # Extract RETURN VALUE section (between header and next section header)
        rv_match = re.search(
            r"^RETURN VALUE\n(.*?)(?=^(?:VALID DURING|EXAMPLES|HINTS|SEE ALSO|CHANGE LOG)\b)",
            block, re.MULTILINE | re.DOTALL,
        )
        if rv_match:
            rv = rv_match.group(1).strip()
            rv = " ".join(rv.split())  # collapse whitespace
            # Filter out entries that accidentally grabbed the next section
            if (rv and rv.lower() not in ("none", "n/a", "")
                    and not rv.startswith("VALID DURING")):
                entry["return_value"] = rv[:300]

        # Extract VALID DURING section (factual event list)
        vd_match = re.search(
            r"^VALID DURING\n\s*(.+?)(?=\n\n|^EXAMPLES|^HINTS|^SEE ALSO|^CHANGE LOG)",
            block, re.MULTILINE | re.DOTALL,
        )
        if vd_match:
            events = vd_match.group(1).strip()
            events = ", ".join(events.split(", "))  # normalize
            if events:
                entry["valid_during"] = events

        if entry:
            entries[name] = entry

    return entries


def _load_irule_schema_enrichment(schema_dir: Path) -> dict[str, dict[str, str]]:
    """Load F5 iRule schema JSON and return {command_name: {description, examples, returnValue}}."""
    if not schema_dir.exists():
        return {}

    enrichment: dict[str, dict[str, str]] = {}

    # Load global commands
    global_file = schema_dir / "global_commands.json"
    if global_file.exists():
        data = json.loads(global_file.read_text(encoding="utf-8"))
        for entry in data:
            name = entry.get("commandName", "")
            if name:
                enrichment[name] = {
                    "description": entry.get("description", ""),
                    "examples": entry.get("examples", ""),
                    "return_value": entry.get("returnValue", ""),
                }

    # Load namespace commands
    for ns_file in sorted(schema_dir.glob("namespace_*.json")):
        data = json.loads(ns_file.read_text(encoding="utf-8"))
        ns_name = ns_file.stem.replace("namespace_", "")
        for entry in data:
            cmd_name = entry.get("commandName", "")
            if cmd_name:
                # Namespace commands may be like "HTTP::uri" or just "uri"
                # The commandName field usually includes the namespace
                enrichment[cmd_name] = {
                    "description": entry.get("description", ""),
                    "examples": entry.get("examples", ""),
                    "return_value": entry.get("returnValue", ""),
                }

    return enrichment


# ─── Main pipeline ─────────────────────────────────────────────────


def load_all_specs() -> dict[str, list[CommandSpec]]:
    """Load all command specs from the Python registry, grouped by category."""
    from core.commands.registry.tcl import tcl_command_specs
    from core.commands.registry.irules import irules_command_specs
    from core.commands.registry.iapps import iapps_command_specs
    from core.commands.registry.expect import expect_command_specs
    from core.commands.registry.stdlib import stdlib_command_specs
    from core.commands.registry.tcllib import tcllib_command_specs
    from core.commands.registry.eda_synopsys import synopsys_command_specs
    from core.commands.registry.eda_cadence import cadence_command_specs
    from core.commands.registry.eda_xilinx import xilinx_command_specs
    from core.commands.registry.eda_quartus import quartus_command_specs
    from core.commands.registry.eda_mentor import mentor_command_specs
    from core.commands.registry.eda_sdc_base import sdc_base_command_specs

    return {
        "tcl": list(tcl_command_specs()),
        "irules": list(irules_command_specs()),
        "iapps": list(iapps_command_specs()),
        "expect": list(expect_command_specs()),
        "stdlib": list(stdlib_command_specs()),
        "tcllib": list(tcllib_command_specs()),
        "eda_synopsys": list(synopsys_command_specs()),
        "eda_cadence": list(cadence_command_specs()),
        "eda_xilinx": list(xilinx_command_specs()),
        "eda_quartus": list(quartus_command_specs()),
        "eda_mentor": list(mentor_command_specs()),
        "eda_sdc": list(sdc_base_command_specs()),
    }


def run_generate(
    output_dir: Path,
    validate_only: bool = False,
    tcl_doc_dir: Path | None = None,
    irule_schema_dir: Path | None = None,
) -> None:
    """Main generation pipeline."""
    specs_by_category = load_all_specs()

    # Load enrichment from reference materials
    tcl_enrichment: dict[str, dict[str, str]] = {}
    irule_enrichment: dict[str, dict[str, str]] = {}

    if tcl_doc_dir:
        print(f"Loading Tcl man page enrichment from {tcl_doc_dir}...")
        tcl_enrichment = _load_tcl_manpage_enrichment(tcl_doc_dir)
        print(f"  Loaded enrichment for {len(tcl_enrichment)} commands")

    if irule_schema_dir:
        print(f"Loading iRule schema enrichment from {irule_schema_dir}...")
        irule_enrichment = _load_irule_schema_enrichment(irule_schema_dir)
        print(f"  Loaded enrichment for {len(irule_enrichment)} commands")

    hover_entries: list[CppHoverEntry] = []
    all_commands: list[tuple[str, CppCommandDesc]] = []
    category_files: list[str] = []

    # For iRules, split by namespace prefix
    irules_specs = specs_by_category.pop("irules")
    irules_by_ns: dict[str, list[CommandSpec]] = {
        "irules_http": [],
        "irules_access": [],
        "irules_ssl": [],
        "irules_misc": [],
    }
    for spec in irules_specs:
        name_upper = spec.name.upper()
        if name_upper.startswith("HTTP"):
            irules_by_ns["irules_http"].append(spec)
        elif name_upper.startswith("ACCESS"):
            irules_by_ns["irules_access"].append(spec)
        elif name_upper.startswith(("SSL", "CLIENTSSL", "SERVERSSL")):
            irules_by_ns["irules_ssl"].append(spec)
        else:
            irules_by_ns["irules_misc"].append(spec)

    # Merge irules splits into category map
    specs_by_category.update(irules_by_ns)

    # Load hover enrichment data
    from scripts.registry.hover_enrichment import EXPECT_ENRICHMENT, IAPPS_ENRICHMENT

    # Load Claude-generated F5 content if available (stored outside repo)
    f5_generated_file = Path.home() / "src" / "tcl-lsp-data" / "f5_hover_generated.json"
    f5_generated: dict[str, dict[str, str]] = {}
    if f5_generated_file.exists():
        f5_generated = json.loads(f5_generated_file.read_text(encoding="utf-8"))
        print(f"  Loaded {len(f5_generated)} Claude-generated F5 hover entries")

    # Load Tcl man page enrichment (verbatim OK per BSD license).
    # Load from multiple versions, newest first. Later versions take priority.
    tcl_manpage_enrichment: dict[str, dict[str, str]] = {}
    if tcl_doc_dir is not None:
        tcl_manpage_enrichment = _load_tcl_manpage_enrichment(tcl_doc_dir)

    # Also load from older Tcl versions for commands not in the primary version.
    for fallback_ver in ["8.6.17", "8.5.19", "8.4.20"]:
        fallback_dir = Path.home() / "src" / f"tcl{fallback_ver}" / "doc"
        if fallback_dir.exists() and fallback_dir != tcl_doc_dir:
            fallback = _load_tcl_manpage_enrichment(fallback_dir)
            # Only add commands not already enriched.
            for name, data in fallback.items():
                if name not in tcl_manpage_enrichment:
                    tcl_manpage_enrichment[name] = data

    # Load iRule man index enrichment (return values, event lists)
    irule_manindex: dict[str, dict[str, str]] = {}
    bigip_man_dir = Path.home() / "src" / "bigip-extract" / "man-21.0.0.1-0.0.13"
    if bigip_man_dir.exists():
        print(f"Loading iRule man index from {bigip_man_dir}...")
        irule_manindex = _load_irule_manindex_enrichment(bigip_man_dir)
        print(f"  Loaded enrichment for {len(irule_manindex)} commands")

    # Map category -> enrichment dict
    # Layer: man index (factual) < bulk-generated < manual enrichment
    irules_enrichment = {**irule_manindex, **f5_generated}
    iapps_enrichment = {**f5_generated, **IAPPS_ENRICHMENT}  # manual wins over bulk

    category_enrichment: dict[str, dict[str, dict[str, str]]] = {
        "tcl": tcl_manpage_enrichment,
        "expect": EXPECT_ENRICHMENT,
        "iapps": iapps_enrichment,
        "irules_http": irules_enrichment,
        "irules_access": irules_enrichment,
        "irules_ssl": irules_enrichment,
        "irules_misc": irules_enrichment,
    }

    # Category ordering
    category_order = [
        "tcl", "stdlib", "tcllib", "expect",
        "irules_http", "irules_access", "irules_ssl", "irules_misc",
        "iapps",
        "eda_synopsys", "eda_cadence", "eda_xilinx", "eda_quartus", "eda_mentor", "eda_sdc",
    ]

    for category in category_order:
        specs = specs_by_category.get(category, [])
        if not specs:
            continue

        inc_file = f"commands_{category}.inc"
        category_files.append(inc_file)

        enrichment = category_enrichment.get(category)
        content, commands = generate_category_inc(category, specs, hover_entries, enrichment)
        all_commands.extend(commands)

        if not validate_only:
            inc_path = output_dir / inc_file
            # .inc files are raw content (no header/footer) — included by all_commands.cpp
            inc_path.write_text(content, encoding="utf-8")
            print(f"  {inc_file}: {len(commands)} commands")

    # Generate master file
    master_content = generate_all_commands_cpp(all_commands, hover_entries, category_files)

    if not validate_only:
        master_path = output_dir / "all_commands.cpp"
        master_path.write_text(master_content, encoding="utf-8")
        print(f"  all_commands.cpp: {len(all_commands)} commands, {len(hover_entries)} hover entries")

    # Summary
    total = len(all_commands)
    hovers = sum(1 for h in hover_entries if h.subcmd_name is None)
    sub_hovers = sum(1 for h in hover_entries if h.subcmd_name is not None)
    missing_hover = sum(1 for _, c in all_commands if c.hover_index < 0)

    print(f"\nSummary:")
    print(f"  Total commands: {total}")
    print(f"  Command hovers: {hovers}")
    print(f"  Subcommand hovers: {sub_hovers}")
    print(f"  Commands without hover: {missing_hover}")
    print(f"  Total hover entries: {len(hover_entries)}")

    if validate_only:
        # Verify hover indices are contiguous
        indices = sorted(h.index for h in hover_entries)
        expected = list(range(len(hover_entries)))
        if indices != expected:
            print("ERROR: Hover indices are not contiguous!")
            sys.exit(1)
        print("Validation passed.")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=ROOT / "native" / "src" / "registry" / "generated",
        help="Output directory for generated C++ files",
    )
    parser.add_argument(
        "--validate",
        action="store_true",
        help="Validate only (don't write files)",
    )
    parser.add_argument(
        "--tcl-doc-dir",
        type=Path,
        default=None,
        help="Tcl man page directory (e.g., ~/src/tcl9.0.3/doc)",
    )
    parser.add_argument(
        "--irule-schema-dir",
        type=Path,
        default=None,
        help="iRule schema split directory (e.g., ~/src/bigip-extract/irule-schema-split)",
    )
    args = parser.parse_args()

    # Auto-detect reference material paths
    tcl_doc = args.tcl_doc_dir
    if tcl_doc is None:
        for version in ["9.0.3", "9.0.2", "8.6.17", "8.5.19", "8.4.20"]:
            candidate = Path.home() / "src" / f"tcl{version}" / "doc"
            if candidate.exists():
                tcl_doc = candidate
                break

    irule_schema = args.irule_schema_dir
    if irule_schema is None:
        candidate = Path.home() / "src" / "bigip-extract" / "irule-schema-split"
        if candidate.exists():
            irule_schema = candidate

    args.output_dir.mkdir(parents=True, exist_ok=True)

    print("Generating native command registry...")
    run_generate(args.output_dir, args.validate, tcl_doc, irule_schema)


if __name__ == "__main__":
    main()
