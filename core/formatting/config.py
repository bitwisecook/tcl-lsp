"""Formatter configuration with F5 iRules Style Guide defaults.

This module is the single source of truth for all formatter settings.
The ``FORMATTER_SETTINGS_CATALOGUE`` at the bottom drives code generation
for editor extension settings (VS Code, JetBrains, etc.) via
``scripts/generate_editor_settings.py``.
"""

from __future__ import annotations

from dataclasses import dataclass, fields, replace
from enum import Enum, auto
from typing import Any


class BraceStyle(Enum):
    """Where to place opening braces."""

    K_AND_R = auto()  # end of line (default per F5 style guide)


class IndentStyle(Enum):
    """What characters to use for indentation."""

    SPACES = auto()
    TABS = auto()


class DocstringStyle(Enum):
    """Where docstrings are placed relative to the proc."""

    PRECEDING = auto()  # comment block above the proc statement
    BODY = auto()  # comment block at the start of the proc body
    NONE = auto()  # do not generate/reformat docstrings


class DocstringTagStyle(Enum):
    """Tag format used in docstrings."""

    DOXYGEN = auto()  # @param, @return, @brief
    PLAIN = auto()  # plain prose (no tags)
    NONE = auto()  # leave tags as-is


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

    # Docstrings
    docstring_style: DocstringStyle = DocstringStyle.NONE
    docstring_tag_style: DocstringTagStyle = DocstringTagStyle.DOXYGEN
    docstring_decoration: bool = False
    docstring_decoration_char: str = "."
    docstring_decoration_width: int = 70

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
        kwargs = {}
        for f in fields(cls):
            if f.name in data:
                val = data[f.name]
                if f.name in _ENUM_FIELDS and isinstance(val, str):
                    val = _ENUM_FIELDS[f.name][val.upper()]
                kwargs[f.name] = val
        return cls(**kwargs)

    def replace(self, **changes) -> FormatterConfig:
        """Return a copy with the given fields replaced."""
        return replace(self, **changes)


# Enum lookup table used by from_dict (auto-discovered from type hints)
_ENUM_FIELDS: dict[str, type[Enum]] = {
    f.name: f.type
    for f in fields(FormatterConfig)
    if isinstance(f.type, type) and issubclass(f.type, Enum)
}
# Fallback: the above only works if type annotations are resolved.
# Explicitly register for safety.
_ENUM_FIELDS.update(
    {
        "brace_style": BraceStyle,
        "indent_style": IndentStyle,
        "docstring_style": DocstringStyle,
        "docstring_tag_style": DocstringTagStyle,
    }
)


# Editor settings catalogue
#
# This is the single source of truth for formatter settings exposed in
# editor extensions.  Each entry is a (field_name, editor_spec) pair
# where editor_spec provides the JSON schema, description, and any
# enum/range constraints.  The codegen script reads this catalogue to
# generate VS Code package.json, JetBrains settings panels, etc.
#
# Structure: list of (section_title, [(field_name, spec), ...])


@dataclass(frozen=True, slots=True)
class SettingSpec:
    """Metadata for a single formatter setting in editor extensions."""

    field_name: str
    json_type: str  # "boolean", "integer", "string"
    description: str
    enum_values: list[str] | None = None
    enum_descriptions: list[str] | None = None
    minimum: int | None = None
    maximum: int | None = None

    def to_vscode_schema(self, default_value: Any) -> dict:
        """Build a VS Code JSON schema fragment for this setting."""
        # Resolve default from enum
        if isinstance(default_value, Enum):
            default_value = default_value.name.lower()
        elif default_value == "\n":
            default_value = "lf"
        elif default_value == "\r\n":
            default_value = "crlf"
        elif default_value == "\r":
            default_value = "cr"

        schema: dict[str, Any] = {
            "type": self.json_type,
            "default": default_value,
            "markdownDescription": self.description,
        }
        if self.enum_values is not None:
            schema["enum"] = self.enum_values
        if self.enum_descriptions is not None:
            schema["enumDescriptions"] = self.enum_descriptions
        if self.minimum is not None:
            schema["minimum"] = self.minimum
        if self.maximum is not None:
            schema["maximum"] = self.maximum
        return schema


@dataclass(frozen=True, slots=True)
class SettingsSection:
    """A group of formatter settings under a common editor heading."""

    title: str
    settings: tuple[SettingSpec, ...]


def _s(
    field_name: str,
    json_type: str,
    description: str,
    **kwargs: Any,
) -> SettingSpec:
    """Shorthand constructor for ``SettingSpec``."""
    return SettingSpec(
        field_name=field_name, json_type=json_type, description=description, **kwargs
    )


FORMATTER_SETTINGS_CATALOGUE: tuple[SettingsSection, ...] = (
    SettingsSection(
        "Formatting — Indentation",
        (
            _s(
                "indent_size",
                "integer",
                "Number of spaces per indentation level.",
                minimum=1,
                maximum=16,
            ),
            _s(
                "indent_style",
                "string",
                "Use spaces or tabs for indentation.",
                enum_values=["spaces", "tabs"],
            ),
            _s(
                "continuation_indent",
                "integer",
                "Extra indentation for continuation lines.",
                minimum=1,
                maximum=16,
            ),
        ),
    ),
    SettingsSection(
        "Formatting — Braces & Style",
        (
            _s(
                "brace_style",
                "string",
                "Brace placement style. K&R places the opening brace at the end of the line.",
                enum_values=["k_and_r"],
            ),
            _s(
                "space_between_braces",
                "boolean",
                "Insert spaces inside single-line braces: `{ body }` instead of `{body}`.",
            ),
            _s(
                "enforce_braced_variables", "boolean", "Rewrite `$var` as `${var}` for consistency."
            ),
            _s(
                "enforce_braced_expr",
                "boolean",
                "Rewrite unbraced `expr` arguments as braced expressions.",
            ),
        ),
    ),
    SettingsSection(
        "Formatting — Line Length & Wrapping",
        (
            _s(
                "max_line_length",
                "integer",
                "Hard limit for line length. Lines exceeding this are wrapped.",
                minimum=40,
            ),
            _s(
                "goal_line_length",
                "integer",
                "Soft target for line length. The formatter prefers to stay within this limit.",
                minimum=40,
            ),
            _s(
                "expand_single_line_bodies",
                "boolean",
                "Expand single-line command bodies onto multiple lines.",
            ),
            _s(
                "min_body_commands_for_expansion",
                "integer",
                "Minimum commands in a body before it is expanded to multiple lines.",
                minimum=1,
            ),
        ),
    ),
    SettingsSection(
        "Formatting — Whitespace & Comments",
        (
            _s("space_after_comment_hash", "boolean", "Ensure a space after `#` in comments."),
            _s("trim_trailing_whitespace", "boolean", "Remove trailing whitespace from lines."),
            _s(
                "align_comments_to_code", "boolean", "Align inline comments to a consistent column."
            ),
            _s(
                "replace_semicolons_with_newlines",
                "boolean",
                "Replace `;` command separators with newlines.",
            ),
        ),
    ),
    SettingsSection(
        "Formatting — Blank Lines & File Format",
        (
            _s(
                "blank_lines_between_procs",
                "integer",
                "Number of blank lines between proc definitions.",
                minimum=0,
                maximum=5,
            ),
            _s(
                "blank_lines_between_blocks",
                "integer",
                "Number of blank lines between top-level blocks.",
                minimum=0,
                maximum=5,
            ),
            _s(
                "max_consecutive_blank_lines",
                "integer",
                "Maximum number of consecutive blank lines to keep.",
                minimum=1,
                maximum=10,
            ),
            _s(
                "line_ending",
                "string",
                "Line ending style for formatted output.",
                enum_values=["lf", "crlf", "cr"],
            ),
            _s("ensure_final_newline", "boolean", "Ensure the file ends with a newline character."),
        ),
    ),
    SettingsSection(
        "Formatting — Docstrings",
        (
            _s(
                "docstring_style",
                "string",
                "Where docstrings are placed relative to `proc` definitions. Set to `none` to leave existing docstrings as-is.",
                enum_values=["preceding", "body", "none"],
                enum_descriptions=[
                    "Comment block above the proc statement.",
                    "Comment block at the start of the proc body.",
                    "Do not generate or reformat docstrings.",
                ],
            ),
            _s(
                "docstring_tag_style",
                "string",
                "Tag format used in docstrings for parameter and return documentation.",
                enum_values=["doxygen", "plain", "none"],
                enum_descriptions=[
                    "Doxygen-style tags: @param, @return, @brief.",
                    "Plain prose with an Arguments: section.",
                    "Leave tag format as-is.",
                ],
            ),
            _s(
                "docstring_decoration",
                "boolean",
                "Add decoration border lines (e.g. `# ......`) around docstrings.",
            ),
            _s(
                "docstring_decoration_char",
                "string",
                "Character used for docstring decoration borders.",
                enum_values=[".", "-", "=", "*", "~"],
            ),
            _s(
                "docstring_decoration_width",
                "integer",
                "Width of docstring decoration border lines.",
                minimum=20,
                maximum=120,
            ),
        ),
    ),
)
