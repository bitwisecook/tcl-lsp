"""Tests for the shared docstring parsing, generation, and formatting module."""

from __future__ import annotations

import textwrap

from core.formatting.docstring import (
    DocstringInfo,
    DocstringTagStyle,
    ParamDoc,
    extract_body_docstring,
    format_docstring,
    generate_stub,
    parse_docstring,
    render_comment_block,
    render_markdown,
)


class TestParseDocstring:
    def test_empty(self):
        info = parse_docstring("")
        assert info.is_empty

    def test_plain_text(self):
        info = parse_docstring("Hello world")
        assert info.description == "Hello world"
        assert info.brief == ""
        assert info.params == []
        assert info.returns == ""

    def test_multiline_description(self):
        info = parse_docstring("Line one\nLine two")
        assert info.description == "Line one\nLine two"

    def test_brief_tag(self):
        info = parse_docstring("@brief Calculate the sum")
        assert info.brief == "Calculate the sum"

    def test_param_with_description(self):
        info = parse_docstring("@param name - The user's name")
        assert len(info.params) == 1
        assert info.params[0].name == "name"
        assert info.params[0].description == "The user's name"

    def test_param_with_dash_separator(self):
        info = parse_docstring("@param filename - Filename to pkg file")
        assert info.params[0].name == "filename"
        assert info.params[0].description == "Filename to pkg file"

    def test_param_without_description(self):
        info = parse_docstring("@param x")
        assert info.params[0].name == "x"
        assert info.params[0].description == ""

    def test_return_tag(self):
        info = parse_docstring("@return The sum of a and b")
        assert info.returns == "The sum of a and b"

    def test_returns_tag(self):
        info = parse_docstring("@returns The result")
        assert info.returns == "The result"

    def test_full_doxygen(self):
        doc = textwrap.dedent("""\
            @brief Calculate the sum
            @param a - First number
            @param b - Second number
            @return The sum of a and b
        """).strip()
        info = parse_docstring(doc)
        assert info.brief == "Calculate the sum"
        assert len(info.params) == 2
        assert info.params[0].name == "a"
        assert info.params[1].name == "b"
        assert info.returns == "The sum of a and b"

    def test_mixed_description_and_tags(self):
        doc = "Update build date and time in pkg\n\n@param filename - Filename to pkg file"
        info = parse_docstring(doc)
        assert "Update build date" in info.description
        assert info.params[0].name == "filename"

    def test_param_by_name(self):
        info = DocstringInfo(params=[ParamDoc("a", "first"), ParamDoc("b", "second")])
        found = info.param_by_name("a")
        assert found is not None
        assert found.description == "first"
        assert info.param_by_name("c") is None


class TestRenderMarkdown:
    def test_simple(self):
        info = DocstringInfo(brief="Hello")
        md = render_markdown(info)
        assert md == "Hello"

    def test_with_params(self):
        info = DocstringInfo(
            brief="Add numbers",
            params=[ParamDoc("a", "First"), ParamDoc("b", "Second")],
            returns="The sum",
        )
        md = render_markdown(info)
        assert "**Parameters:**" in md
        assert "**a** \u2014 First" in md
        assert "**Returns:** The sum" in md


class TestFormatDocstring:
    def test_roundtrip(self):
        doc = "@brief Hello\n@param x - A number\n@return The result"
        md = format_docstring(doc)
        assert "Hello" in md
        assert "**x**" in md
        assert "**Returns:**" in md


class TestRenderCommentBlock:
    def test_doxygen_style(self):
        info = DocstringInfo(
            brief="Say hello",
            params=[ParamDoc("name", "Who to greet")],
            returns="Greeting string",
        )
        block = render_comment_block(info, tag_style=DocstringTagStyle.DOXYGEN)
        assert "# @brief Say hello" in block
        assert "# @param name - Who to greet" in block
        assert "# @return Greeting string" in block

    def test_plain_style(self):
        info = DocstringInfo(
            brief="Say hello",
            params=[ParamDoc("name", "Who to greet")],
            returns="Greeting string",
        )
        block = render_comment_block(info, tag_style=DocstringTagStyle.PLAIN)
        assert "# Say hello" in block
        assert "# Arguments:" in block
        assert "#   name - Who to greet" in block
        assert "# Returns: Greeting string" in block

    def test_with_decoration(self):
        info = DocstringInfo(brief="Hello")
        block = render_comment_block(
            info, decoration=True, decoration_char=".", decoration_width=20
        )
        lines = block.splitlines()
        assert lines[0] == "# " + "." * 20
        assert lines[-1] == "# " + "." * 20

    def test_with_indent(self):
        info = DocstringInfo(brief="Hello")
        block = render_comment_block(info, indent="    ")
        assert block.startswith("    # ")


class TestGenerateStub:
    def test_basic(self):
        stub = generate_stub("greet", ["name"])
        assert "# @brief TODO: describe greet" in stub
        assert "# @param name" in stub

    def test_with_defaults(self):
        stub = generate_stub("greet", ["name", "greeting"], {"greeting": "Hello"})
        assert "(default: Hello)" in stub

    def test_args_param(self):
        stub = generate_stub("foo", ["args"])
        assert "Additional arguments" in stub

    def test_plain_style(self):
        stub = generate_stub("foo", ["x"], tag_style=DocstringTagStyle.PLAIN)
        assert "# Arguments:" in stub
        assert "#   x" in stub

    def test_decoration(self):
        stub = generate_stub("foo", ["x"], decoration=True)
        lines = stub.splitlines()
        assert "." in lines[0]


class TestExtractBodyDocstring:
    def test_empty_body(self):
        assert extract_body_docstring("") == ""

    def test_no_comments(self):
        assert extract_body_docstring("puts hello") == ""

    def test_simple_comment(self):
        assert extract_body_docstring("# Hello\nputs hello") == "Hello"

    def test_multiline_comment(self):
        body = "# Line one\n# Line two\nputs hello"
        assert extract_body_docstring(body) == "Line one\nLine two"

    def test_decoration_stripped(self):
        body = "# ....\n# Hello\n# ....\nputs hello"
        assert extract_body_docstring(body) == "Hello"

    def test_hash_decoration_stripped(self):
        body = "########\n# Hello\n########\nputs hello"
        assert extract_body_docstring(body) == "Hello"

    def test_blank_line_stops(self):
        body = "# First block\n\n# Second block\nputs hello"
        assert extract_body_docstring(body) == "First block"

    def test_leading_blank_lines_skipped(self):
        body = "\n\n# Hello\nputs hello"
        assert extract_body_docstring(body) == "Hello"

    def test_osvvm_style(self):
        body = textwrap.dedent("""\
            # ....................................................................
            # Update build date and time in pkg
            #
            # @param filename - Filename to pkg file
            # ....................................................................
            puts $filename
        """)
        result = extract_body_docstring(body)
        assert "Update build date" in result
        assert "@param filename" in result


class TestDocstringInfoDict:
    def test_to_dict(self):
        info = DocstringInfo(
            brief="Hello",
            params=[ParamDoc("x", "A number")],
            returns="Result",
        )
        d = info.to_dict()
        assert d["brief"] == "Hello"
        assert d["params"] == [{"name": "x", "description": "A number"}]
        assert d["returns"] == "Result"

    def test_empty_to_dict(self):
        info = DocstringInfo()
        d = info.to_dict()
        assert d == {}
