"""Tcl source code formatter."""

from .config import BraceStyle, DocstringStyle, DocstringTagStyle, FormatterConfig, IndentStyle
from .docstring import (
    DocstringInfo,
    ParamDoc,
    extract_body_docstring,
    format_docstring,
    generate_stub,
    generate_stub_for_proc,
    parse_docstring,
    render_comment_block,
    render_markdown,
    resolve_tag_style,
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
    "generate_stub_for_proc",
    "parse_docstring",
    "render_comment_block",
    "render_markdown",
    "resolve_tag_style",
]
