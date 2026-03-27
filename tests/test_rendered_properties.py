"""Tests for the Rendered Value Properties analysis pass."""

from core.compiler.rendered_properties import (
    RenderedProperties,
    RenderedValueProps,
    _evaluate_rendered_props_for_const,
    _evaluate_rendered_props_for_value,
    rendered_join,
)


class TestRenderedJoin:
    """Join semantics: may=union, must=intersection."""

    def test_may_union(self):
        a = RenderedValueProps(may=RenderedProperties.HAS_FORWARD_SLASH)
        b = RenderedValueProps(may=RenderedProperties.HAS_CRLF)
        joined = rendered_join(a, b)
        assert joined.may & RenderedProperties.HAS_FORWARD_SLASH
        assert joined.may & RenderedProperties.HAS_CRLF

    def test_must_intersection(self):
        a = RenderedValueProps(
            must=RenderedProperties.STARTS_WITH_SLASH | RenderedProperties.STARTS_WITH_DASH
        )
        b = RenderedValueProps(must=RenderedProperties.STARTS_WITH_SLASH)
        joined = rendered_join(a, b)
        assert joined.must & RenderedProperties.STARTS_WITH_SLASH
        assert not (joined.must & RenderedProperties.STARTS_WITH_DASH)

    def test_phi_loses_must_when_one_branch_disagrees(self):
        a = RenderedValueProps(must=RenderedProperties.STARTS_WITH_SLASH)
        b = RenderedValueProps(must=RenderedProperties.NONE)
        joined = rendered_join(a, b)
        assert not (joined.must & RenderedProperties.STARTS_WITH_SLASH)


class TestEvaluateConst:
    """Properties of constant (IRAssignConst) values."""

    def test_slash_in_constant(self):
        props = _evaluate_rendered_props_for_const("/etc/config")
        assert props.may & RenderedProperties.HAS_FORWARD_SLASH
        assert props.must & RenderedProperties.STARTS_WITH_SLASH

    def test_dash_prefix(self):
        props = _evaluate_rendered_props_for_const("-verbose")
        assert props.must & RenderedProperties.STARTS_WITH_DASH

    def test_plain_string_no_path(self):
        props = _evaluate_rendered_props_for_const("hello")
        assert not (props.may & RenderedProperties.HAS_FORWARD_SLASH)
        assert not (props.may & RenderedProperties.HAS_BACKSLASH)
        assert not (props.must & RenderedProperties.STARTS_WITH_SLASH)

    def test_null_detection(self):
        props = _evaluate_rendered_props_for_const("test\x00data")
        assert props.may & RenderedProperties.HAS_NULL

    def test_crlf_detection(self):
        props = _evaluate_rendered_props_for_const("line1\nline2")
        assert props.may & RenderedProperties.HAS_CRLF


class TestEvaluateValue:
    """Properties of interpolated (IRAssignValue) values."""

    def test_var_with_forward_slash(self):
        """$dir/file.txt -> HAS_FORWARD_SLASH + HAS_INTERPOLATION."""
        props = _evaluate_rendered_props_for_value('"$dir/file.txt"')
        assert props.may & RenderedProperties.HAS_FORWARD_SLASH
        assert props.may & RenderedProperties.HAS_INTERPOLATION

    def test_pure_var_ref(self):
        """$x -> HAS_INTERPOLATION, no path bits."""
        props = _evaluate_rendered_props_for_value("$x")
        assert props.may & RenderedProperties.HAS_INTERPOLATION
        assert not (props.may & RenderedProperties.HAS_FORWARD_SLASH)
        assert not (props.must & RenderedProperties.STARTS_WITH_SLASH)

    def test_pure_cmd_subst(self):
        """[pwd] -> HAS_INTERPOLATION + HAS_FORWARD_SLASH (path-returning)."""
        props = _evaluate_rendered_props_for_value("[pwd]")
        assert props.may & RenderedProperties.HAS_INTERPOLATION
        assert props.may & RenderedProperties.HAS_FORWARD_SLASH

    def test_file_dirname_returns_path(self):
        props = _evaluate_rendered_props_for_value("[file dirname $x]")
        assert props.may & RenderedProperties.HAS_FORWARD_SLASH
        assert props.may & RenderedProperties.HAS_INTERPOLATION

    def test_non_path_cmd_no_slash(self):
        """[string length $x] -> no HAS_FORWARD_SLASH."""
        props = _evaluate_rendered_props_for_value("[string length $x]")
        assert props.may & RenderedProperties.HAS_INTERPOLATION
        assert not (props.may & RenderedProperties.HAS_FORWARD_SLASH)

    def test_leading_slash_with_var(self):
        """/static/$var -> STARTS_WITH_SLASH + HAS_FORWARD_SLASH."""
        props = _evaluate_rendered_props_for_value('"/static/$var"')
        assert props.must & RenderedProperties.STARTS_WITH_SLASH
        assert props.may & RenderedProperties.HAS_FORWARD_SLASH
        assert props.may & RenderedProperties.HAS_INTERPOLATION

    def test_var_at_start_clears_must(self):
        """$dir/file -> no STARTS_WITH_SLASH (variable at start)."""
        props = _evaluate_rendered_props_for_value('"$dir/file"')
        assert not (props.must & RenderedProperties.STARTS_WITH_SLASH)

    # --- Escape rendering ---

    def test_backslash_n_not_path_sep(self):
        r"""$greeting\n$farewell -> \n is newline, not path backslash."""
        props = _evaluate_rendered_props_for_value(r'"$greeting\n$farewell"')
        assert not (props.may & RenderedProperties.HAS_BACKSLASH)
        assert not (props.may & RenderedProperties.HAS_FORWARD_SLASH)
        assert props.may & RenderedProperties.HAS_CRLF  # \n renders to newline

    def test_backslash_t_not_path_sep(self):
        r"""$name\t$value -> \t is tab, not path backslash."""
        props = _evaluate_rendered_props_for_value(r'"$name\t$value"')
        assert not (props.may & RenderedProperties.HAS_BACKSLASH)
        assert not (props.may & RenderedProperties.HAS_FORWARD_SLASH)

    def test_hex_escape_to_slash(self):
        r"""\x2f$var -> \x2f renders to '/', should detect HAS_FORWARD_SLASH."""
        props = _evaluate_rendered_props_for_value(r'"\x2f$var"')
        assert props.may & RenderedProperties.HAS_FORWARD_SLASH
        assert props.must & RenderedProperties.STARTS_WITH_SLASH

    def test_hex_escape_not_slash(self):
        r"""\x61$var -> \x61 renders to 'a', no path separator."""
        props = _evaluate_rendered_props_for_value(r'"\x61$var"')
        assert not (props.may & RenderedProperties.HAS_FORWARD_SLASH)
        assert not (props.may & RenderedProperties.HAS_BACKSLASH)

    def test_double_backslash_is_path_sep(self):
        r"""$dir\\file -> \\ renders to single \, IS a path separator."""
        props = _evaluate_rendered_props_for_value(r'"$dir\\file"')
        assert props.may & RenderedProperties.HAS_BACKSLASH

    def test_null_in_escape(self):
        r"""\x00 in value -> HAS_NULL."""
        props = _evaluate_rendered_props_for_value(r'"\x00$data"')
        assert props.may & RenderedProperties.HAS_NULL

    # --- Double escape detection ---

    def test_double_escape_detected(self):
        r"""\\n in source -> after rendering becomes \n (literal), HAS_DOUBLE_ESCAPE."""
        props = _evaluate_rendered_props_for_value(r'"$x\\n"')
        assert props.may & RenderedProperties.HAS_DOUBLE_ESCAPE

    def test_no_double_escape_for_normal(self):
        r"""\n is a single escape, not double."""
        props = _evaluate_rendered_props_for_value(r'"$x\n"')
        assert not (props.may & RenderedProperties.HAS_DOUBLE_ESCAPE)


class TestFullPassIntegration:
    """Integration test: run the pass via find_taint_warnings."""

    def test_hex_escape_slash_triggers_w201(self):
        r"""\x2f renders to '/' -- should trigger W201."""
        from core.compiler.taint import find_taint_warnings

        ws = [w for w in find_taint_warnings(r'set path "\x2f$var"') if w.code == "W201"]
        assert len(ws) == 1

    def test_hex_escape_a_no_w201(self):
        r"""\x61 renders to 'a' -- should NOT trigger W201."""
        from core.compiler.taint import find_taint_warnings

        ws = [w for w in find_taint_warnings(r'set path "\x61$var"') if w.code == "W201"]
        assert len(ws) == 0
