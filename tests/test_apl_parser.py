"""Tests for the APL (iApp presentation language) parser."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.bigip.apl_parser import AplTokenKind, tokenise_apl


class TestAplTokeniser:
    """Tests for APL tokenisation."""

    def test_comment(self):
        tokens = tokenise_apl("# This is a comment")
        assert any(t.kind == AplTokenKind.COMMENT for t in tokens)

    def test_directive_include(self):
        tokens = tokenise_apl('#include "f5.apl_common"')
        directives = [t for t in tokens if t.kind == AplTokenKind.DIRECTIVE]
        assert len(directives) == 1
        assert directives[0].length == len("#include")

    def test_directive_inline(self):
        tokens = tokenise_apl("#inline some_code")
        directives = [t for t in tokens if t.kind == AplTokenKind.DIRECTIVE]
        assert len(directives) == 1

    def test_section_keyword(self):
        tokens = tokenise_apl("section basic {")
        section_kws = [t for t in tokens if t.kind == AplTokenKind.SECTION_KW]
        assert len(section_kws) == 1
        names = [t for t in tokens if t.kind == AplTokenKind.SECTION_NAME]
        assert len(names) == 1

    def test_text_keyword(self):
        tokens = tokenise_apl("text {")
        section_kws = [t for t in tokens if t.kind == AplTokenKind.SECTION_KW]
        assert len(section_kws) == 1

    def test_table_keyword(self):
        tokens = tokenise_apl("table members {")
        section_kws = [t for t in tokens if t.kind == AplTokenKind.SECTION_KW]
        assert len(section_kws) == 1
        names = [t for t in tokens if t.kind == AplTokenKind.SECTION_NAME]
        assert len(names) == 1

    def test_row_keyword(self):
        tokens = tokenise_apl("    row member {")
        section_kws = [t for t in tokens if t.kind == AplTokenKind.SECTION_KW]
        assert len(section_kws) == 1

    def test_define(self):
        tokens = tokenise_apl("define choice yesno_choice {")
        defines = [t for t in tokens if t.kind == AplTokenKind.DEFINE]
        assert len(defines) == 1
        # "choice" should be highlighted as the base type
        field_types = [t for t in tokens if t.kind == AplTokenKind.FIELD_TYPE]
        assert any(t.char == 7 and t.length == 6 for t in field_types)  # "choice"
        # "yesno_choice" should be highlighted as the define name
        names = [t for t in tokens if t.kind == AplTokenKind.DEFINE_NAME]
        assert len(names) == 1
        assert names[0].char == 14
        assert names[0].length == 12  # len("yesno_choice")

    def test_field_type_string(self):
        tokens = tokenise_apl('    string addr default "0.0.0.0" required')
        field_types = [t for t in tokens if t.kind == AplTokenKind.FIELD_TYPE]
        assert len(field_types) == 1
        field_names = [t for t in tokens if t.kind == AplTokenKind.FIELD_NAME]
        assert len(field_names) == 1

    def test_field_type_choice(self):
        tokens = tokenise_apl('choice protocol display "medium" {')
        field_types = [t for t in tokens if t.kind == AplTokenKind.FIELD_TYPE]
        assert len(field_types) == 1

    def test_field_type_password(self):
        tokens = tokenise_apl('    password pw1 display "large"')
        field_types = [t for t in tokens if t.kind == AplTokenKind.FIELD_TYPE]
        assert len(field_types) == 1

    def test_all_field_types(self):
        """All known field-type keywords are recognised."""
        keywords = [
            "string",
            "choice",
            "editchoice",
            "multichoice",
            "message",
            "password",
            "yesno",
            "noyes",
            "enadis",
            "enadisdry",
            "disena",
            "indefint",
            "falsetrue",
            "truefalse",
            "tcpprof",
        ]
        for kw in keywords:
            tokens = tokenise_apl(f"    {kw} myfield")
            field_types = [t for t in tokens if t.kind == AplTokenKind.FIELD_TYPE]
            assert len(field_types) == 1, f"Expected field type for {kw}"

    def test_attributes(self):
        tokens = tokenise_apl('    string addr default "0.0.0.0" display "large" required')
        attrs = [t for t in tokens if t.kind == AplTokenKind.ATTRIBUTE]
        # default, display, required = 3
        assert len(attrs) == 3

    def test_validator(self):
        tokens = tokenise_apl('    string addr validator "IpAddress"')
        validators = [t for t in tokens if t.kind == AplTokenKind.VALIDATOR_VALUE]
        assert len(validators) == 1

    def test_optional(self):
        tokens = tokenise_apl('optional ( basic.use_snat == "yes" ) {')
        optionals = [t for t in tokens if t.kind == AplTokenKind.OPTIONAL]
        assert len(optionals) == 1

    def test_variable(self):
        tokens = tokenise_apl('message "Value: $my_var"')
        variables = [t for t in tokens if t.kind == AplTokenKind.VARIABLE]
        assert len(variables) == 1

    def test_variable_braced(self):
        tokens = tokenise_apl('message "Value: ${my_var}"')
        variables = [t for t in tokens if t.kind == AplTokenKind.VARIABLE]
        assert len(variables) == 1

    def test_string_literal(self):
        tokens = tokenise_apl('"hello world"')
        strings = [t for t in tokens if t.kind == AplTokenKind.STRING]
        assert len(strings) == 1

    def test_escape_in_string(self):
        tokens = tokenise_apl('"hello\\nworld"')
        escapes = [t for t in tokens if t.kind == AplTokenKind.ESCAPE]
        assert len(escapes) == 1

    def test_operator_arrow(self):
        tokens = tokenise_apl('"Yes" => "yes"')
        ops = [t for t in tokens if t.kind == AplTokenKind.OPERATOR]
        assert len(ops) == 1

    def test_number(self):
        tokens = tokenise_apl("    string port default 443")
        numbers = [t for t in tokens if t.kind == AplTokenKind.NUMBER]
        assert len(numbers) == 1

    def test_multiline_presentation(self):
        """Full APL snippet tokenises without errors."""
        source = (
            "# Example\n"
            '#include "f5.apl_common"\n'
            "\n"
            "define choice yesno {\n"
            '    "Yes" => "yes",\n'
            '    "No" => "no"\n'
            "}\n"
            "\n"
            "section basic {\n"
            '    string addr default "0.0.0.0" required validator "IpAddress"\n'
            "    yesno use_snat\n"
            "}\n"
            "\n"
            'optional ( basic.use_snat == "yes" ) {\n'
            "    string snat_pool required\n"
            "}\n"
            "\n"
            "text {\n"
            '    basic "Basic Config"\n'
            '    basic.addr "IP Address"\n'
            "}\n"
        )
        tokens = tokenise_apl(source)
        kinds = {t.kind for t in tokens}
        assert AplTokenKind.COMMENT in kinds
        assert AplTokenKind.DIRECTIVE in kinds
        assert AplTokenKind.DEFINE in kinds
        assert AplTokenKind.SECTION_KW in kinds
        assert AplTokenKind.FIELD_TYPE in kinds
        assert AplTokenKind.ATTRIBUTE in kinds
        assert AplTokenKind.OPTIONAL in kinds
        assert AplTokenKind.VALIDATOR_VALUE in kinds
        assert AplTokenKind.OPERATOR in kinds
        assert AplTokenKind.STRING in kinds

    def test_comment_not_confused_with_directive(self):
        """A plain # comment should not be marked as a directive."""
        tokens = tokenise_apl("# just a comment")
        directives = [t for t in tokens if t.kind == AplTokenKind.DIRECTIVE]
        assert len(directives) == 0

    def test_token_positions(self):
        """Token positions match the source text."""
        source = "section basic {"
        tokens = tokenise_apl(source)
        section_kw = [t for t in tokens if t.kind == AplTokenKind.SECTION_KW][0]
        assert section_kw.line == 0
        assert section_kw.char == 0
        assert section_kw.length == 7
        assert source[section_kw.char : section_kw.char + section_kw.length] == "section"

        name = [t for t in tokens if t.kind == AplTokenKind.SECTION_NAME][0]
        assert source[name.char : name.char + name.length] == "basic"
