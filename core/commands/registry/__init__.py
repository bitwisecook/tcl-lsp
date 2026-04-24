"""Registry package for command metadata."""

from __future__ import annotations

from .command_registry import REGISTRY, CommandRegistry
from .info import CommandInfo, EventInfo, lookup_command_info, lookup_event_info
from .models import (
    ArgumentValueSpec,
    CommandLegality,
    CommandSpec,
    DialectStatus,
    EmitContext,
    EventCommandSet,
    FormatType,
    FormSpec,
    HoverSnippet,
    KeywordCompletion,
    OptionSpec,
    PatternType,
    SubCommand,
    ValidationSpec,
    WasmEmitHook,
)

__all__ = [
    "ArgumentValueSpec",
    "CommandLegality",
    "CommandRegistry",
    "CommandSpec",
    "CommandInfo",
    "DialectStatus",
    "EmitContext",
    "EventInfo",
    "EventCommandSet",
    "FormatType",
    "FormSpec",
    "HoverSnippet",
    "KeywordCompletion",
    "OptionSpec",
    "PatternType",
    "REGISTRY",
    "SubCommand",
    "ValidationSpec",
    "WasmEmitHook",
    "lookup_command_info",
    "lookup_event_info",
]
