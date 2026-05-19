"""Shared docstring parsing, generation, and formatting for Tcl procs.

Provides a structured ``DocstringInfo`` model and utilities for converting
between raw comment text and structured documentation.  Used by the
formatter, LSP features (hover, completion, signature help), and AI tools.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from .config import DocstringTagStyle

if TYPE_CHECKING:
    from analyser.semantic_model import ProcDef


@dataclass(frozen=True, slots=True)
class ParamDoc:
    """Documentation for a single parameter."""

    name: str
    description: str = ""


@dataclass(slots=True)
class DocstringInfo:
    """Structured representation of a proc docstring.

    Parsed from raw comment text via ``parse_docstring`` and rendered
    back to comment text via ``render_docstring`` or to markdown via
    ``render_markdown``.
    """

    brief: str = ""
    description: str = ""
    params: list[ParamDoc] = field(default_factory=list)
    returns: str = ""

    @property
    def is_empty(self) -> bool:
        return not self.brief and not self.description and not self.params and not self.returns

    def param_by_name(self, name: str) -> ParamDoc | None:
        """Look up a parameter doc by name."""
        for p in self.params:
            if p.name == name:
                return p
        return None

    def to_dict(self) -> dict:
        """Serialise to a JSON-friendly dict for AI tools."""
        result: dict = {}
        if self.brief:
            result["brief"] = self.brief
        if self.description:
            result["description"] = self.description
        if self.params:
            result["params"] = [{"name": p.name, "description": p.description} for p in self.params]
        if self.returns:
            result["returns"] = self.returns
        return result


# Parsing

_DECORATION_CHARS = frozenset(".-=*~#")


def parse_docstring(text: str) -> DocstringInfo:
    """Parse a raw docstring into structured form.

    Recognises ``@param``, ``@return``/``@returns``, and ``@brief`` tags.
    Lines that do not match any tag are accumulated as description text.
    """
    brief = ""
    description_lines: list[str] = []
    params: list[ParamDoc] = []
    returns_parts: list[str] = []

    for line in text.splitlines():
        stripped = line.strip()
        low = stripped.lower()

        if low.startswith("@param ") or low.startswith("@param\t"):
            rest = stripped[7:].strip()
            parts = rest.split(None, 1)
            if len(parts) == 2:
                name = parts[0].rstrip(" -")
                desc = parts[1].lstrip("- ")
                params.append(ParamDoc(name=name, description=desc))
            elif parts:
                params.append(ParamDoc(name=parts[0]))

        elif low.startswith(("@return ", "@return\t", "@returns ", "@returns\t")):
            rest = stripped.split(None, 1)[1] if (" " in stripped or "\t" in stripped) else ""
            returns_parts.append(rest.lstrip("- "))

        elif low.startswith("@brief ") or low.startswith("@brief\t"):
            brief = stripped[7:].strip()

        else:
            # Skip decoration-only lines (dots, dashes, hashes, etc.)
            if stripped and all(ch in _DECORATION_CHARS for ch in stripped):
                continue
            description_lines.append(stripped)

    desc = "\n".join(description_lines).strip()

    return DocstringInfo(
        brief=brief,
        description=desc,
        params=params,
        returns=" ".join(returns_parts),
    )


# Rendering to markdown (for LSP hover / completion / signature help)


def render_markdown(info: DocstringInfo) -> str:
    """Render a ``DocstringInfo`` as markdown for LSP display."""
    parts: list[str] = []

    # Brief goes first
    if info.brief:
        parts.append(info.brief)

    # Description
    if info.description:
        parts.append(info.description)

    # Parameters
    if info.params:
        lines = ["**Parameters:**"]
        for p in info.params:
            if p.description:
                lines.append(f"- **{p.name}** \u2014 {p.description}")
            else:
                lines.append(f"- **{p.name}**")
        parts.append("\n".join(lines))

    # Returns
    if info.returns:
        parts.append(f"**Returns:** {info.returns}")

    return "\n\n".join(parts)


def format_docstring(text: str) -> str:
    """Parse a raw docstring and render it as markdown.

    Convenience wrapper combining ``parse_docstring`` and ``render_markdown``.
    """
    return render_markdown(parse_docstring(text))


# Rendering to Tcl comment text (for formatter / code actions)


def render_comment_block(
    info: DocstringInfo,
    *,
    tag_style: DocstringTagStyle = DocstringTagStyle.DOXYGEN,
    decoration: bool = False,
    decoration_char: str = ".",
    decoration_width: int = 70,
    indent: str = "",
) -> str:
    """Render a ``DocstringInfo`` as a Tcl comment block.

    Returns a string of ``# ...`` lines (without trailing newline)
    ready for insertion into Tcl source.
    """
    lines: list[str] = []

    if decoration:
        lines.append(f"{indent}# {decoration_char * decoration_width}")

    if tag_style is DocstringTagStyle.DOXYGEN:
        if info.brief:
            lines.append(f"{indent}# @brief {info.brief}")
        if info.description:
            for desc_line in info.description.splitlines():
                lines.append(f"{indent}# {desc_line}" if desc_line else f"{indent}#")
        if info.params:
            for p in info.params:
                if p.description:
                    lines.append(f"{indent}# @param {p.name} - {p.description}")
                else:
                    lines.append(f"{indent}# @param {p.name}")
        if info.returns:
            lines.append(f"{indent}# @return {info.returns}")
    else:
        # Plain style (also used for NONE — leave tags as-is falls through here).
        if info.brief:
            lines.append(f"{indent}# {info.brief}")
        if info.description:
            for desc_line in info.description.splitlines():
                lines.append(f"{indent}# {desc_line}" if desc_line else f"{indent}#")
        if info.params:
            lines.append(f"{indent}#")
            lines.append(f"{indent}# Arguments:")
            for p in info.params:
                if p.description:
                    lines.append(f"{indent}#   {p.name} - {p.description}")
                else:
                    lines.append(f"{indent}#   {p.name}")
        if info.returns:
            lines.append(f"{indent}#")
            lines.append(f"{indent}# Returns: {info.returns}")

    if decoration:
        lines.append(f"{indent}# {decoration_char * decoration_width}")

    return "\n".join(lines)


# Generating a docstring stub from proc signature


def generate_stub(
    proc_name: str,
    param_names: list[str],
    param_defaults: dict[str, str] | None = None,
    *,
    tag_style: DocstringTagStyle = DocstringTagStyle.DOXYGEN,
    decoration: bool = False,
    decoration_char: str = ".",
    decoration_width: int = 70,
    indent: str = "",
) -> str:
    """Generate a docstring stub for a proc from its signature.

    Creates a template with placeholder descriptions for each parameter.
    """
    defaults = param_defaults or {}
    params = []
    for name in param_names:
        if name == "args":
            params.append(ParamDoc(name=name, description="Additional arguments"))
        elif name in defaults:
            params.append(ParamDoc(name=name, description=f"(default: {defaults[name]})"))
        else:
            params.append(ParamDoc(name=name, description=""))

    info = DocstringInfo(
        brief=f"TODO: describe {proc_name}",
        params=params,
    )

    return render_comment_block(
        info,
        tag_style=tag_style,
        decoration=decoration,
        decoration_char=decoration_char,
        decoration_width=decoration_width,
        indent=indent,
    )


# Extracting body docstring from proc body text


def extract_body_docstring(body: str) -> str:
    """Extract the leading comment block from a proc body.

    Returns the accumulated comment text (lines joined with newlines) if
    the body starts with one or more comment lines, otherwise returns an
    empty string.  Decoration lines consisting only of dots, dashes,
    hashes, or similar characters are stripped.
    """
    lines: list[str] = []
    for raw_line in body.splitlines():
        stripped = raw_line.strip()
        if not stripped:
            if lines:
                break
            continue
        if stripped.startswith("#"):
            text = stripped.lstrip("#").strip()
            # Skip hash-only decoration lines
            if not text and set(stripped) <= {"#"}:
                continue
            # Skip decoration lines (dots, dashes, equals, etc.)
            if text and all(ch in _DECORATION_CHARS for ch in text):
                continue
            lines.append(text)
        else:
            break
    return "\n".join(lines)


def resolve_tag_style(style: str) -> DocstringTagStyle:
    """Resolve a tag-style string to the corresponding enum value.

    Case-insensitive.  Unrecognised values default to ``DOXYGEN``.
    """
    if style.lower() == "plain":
        return DocstringTagStyle.PLAIN
    return DocstringTagStyle.DOXYGEN


def generate_stub_for_proc(
    proc_def: ProcDef,
    *,
    tag_style: DocstringTagStyle = DocstringTagStyle.DOXYGEN,
    decoration: bool = False,
    decoration_char: str = ".",
    decoration_width: int = 70,
    indent: str = "",
) -> str:
    """Generate a docstring stub from a ``ProcDef``.

    Convenience wrapper around ``generate_stub`` that extracts parameter
    names and defaults from the proc definition automatically.
    """
    param_names = [p.name for p in proc_def.params]
    param_defaults = {p.name: p.default_value for p in proc_def.params if p.has_default}
    return generate_stub(
        proc_def.name,
        param_names,
        param_defaults,
        tag_style=tag_style,
        decoration=decoration,
        decoration_char=decoration_char,
        decoration_width=decoration_width,
        indent=indent,
    )
