"""Formatter configuration with F5 iRules Style Guide defaults."""

from __future__ import annotations

from dataclasses import dataclass, fields, replace
from enum import Enum, auto


class BraceStyle(Enum):
    """Where to place opening braces."""

    K_AND_R = auto()  # end of line (default per F5 style guide)


class IndentStyle(Enum):
    """What characters to use for indentation."""

    SPACES = auto()
    TABS = auto()


@dataclass
class FormatterConfig:
    """All configurable formatting options.

    Default values follow the F5 iRules Style Guide:
    https://community.f5.com/kb/technicalarticles/irules-style-guide/305921
    """

    # Indentation
    indent_size: int = 4
    indent_style: IndentStyle = IndentStyle.SPACES
    continuation_indent: int = 4

    # Brace placement
    brace_style: BraceStyle = BraceStyle.K_AND_R
    space_between_braces: bool = True

    # Line length
    max_line_length: int = 120
    goal_line_length: int = 100

    # Spacing
    space_after_comment_hash: bool = True
    trim_trailing_whitespace: bool = True

    # Comments
    align_comments_to_code: bool = True

    # Variables
    enforce_braced_variables: bool = False

    # Expressions
    enforce_braced_expr: bool = False

    # File format
    line_ending: str = "\n"
    ensure_final_newline: bool = True

    # Commands
    expand_single_line_bodies: bool = False
    min_body_commands_for_expansion: int = 2

    # Blank lines
    blank_lines_between_procs: int = 1
    blank_lines_between_blocks: int = 1
    max_consecutive_blank_lines: int = 2

    # Semicolons
    replace_semicolons_with_newlines: bool = True

    def to_dict(self) -> dict:
        """Serialise to a dict suitable for JSON/LSP settings."""
        result = {}
        for f in fields(self):
            val = getattr(self, f.name)
            if isinstance(val, Enum):
                result[f.name] = val.name.lower()
            else:
                result[f.name] = val
        return result

    @classmethod
    def from_dict(cls, data: dict) -> FormatterConfig:
        """Deserialise from a dict (e.g., LSP workspace settings)."""
        enum_fields = {
            "brace_style": BraceStyle,
            "indent_style": IndentStyle,
        }
        kwargs = {}
        for f in fields(cls):
            if f.name in data:
                val = data[f.name]
                if f.name in enum_fields and isinstance(val, str):
                    val = enum_fields[f.name][val.upper()]
                kwargs[f.name] = val
        return cls(**kwargs)

    def replace(self, **changes) -> FormatterConfig:
        """Return a copy with the given fields replaced."""
        return replace(self, **changes)
