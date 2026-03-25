"""Tcl source code formatter."""

from .config import BraceStyle, DocstringStyle, DocstringTagStyle, FormatterConfig, IndentStyle
from .docstring import (
    DocstringInfo,
    ParamDoc,
    extract_body_docstring,
    format_docstring,
    generate_stub,
    parse_docstring,
    render_comment_block,
    render_markdown,
)
from .formatter import format_tcl

__all__ = [
    "BraceStyle",
    "DocstringInfo",
    "DocstringStyle",
    "DocstringTagStyle",
    "FormatterConfig",
    "IndentStyle",
    "ParamDoc",
    "extract_body_docstring",
    "format_docstring",
    "format_tcl",
    "generate_stub",
    "parse_docstring",
    "render_comment_block",
    "render_markdown",
]
