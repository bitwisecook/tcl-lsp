"""Registry package for command metadata."""

from __future__ import annotations

from .command_registry import REGISTRY, CommandRegistry
from .info import CommandInfo, EventInfo, lookup_command_info, lookup_event_info
from .models import (
    ArgumentValueSpec,
    CommandLegality,
    CommandSpec,
    DialectStatus,
    EventCommandSet,
    FormatType,
    FormSpec,
    HoverSnippet,
    KeywordCompletion,
    OptionSpec,
    PatternType,
    SubCommand,
    ValidationSpec,
)

__all__ = [
    "ArgumentValueSpec",
    "CommandLegality",
    "CommandRegistry",
    "CommandSpec",
    "CommandInfo",
    "DialectStatus",
    "EventInfo",
    "EventCommandSet",
    "FormatType",
    "FormSpec",
    "HoverSnippet",
    "KeywordCompletion",
    "OptionSpec",
    "PatternType",
    "REGISTRY",
    "lookup_command_info",
    "lookup_event_info",
    "SubCommand",
    "ValidationSpec",
]
