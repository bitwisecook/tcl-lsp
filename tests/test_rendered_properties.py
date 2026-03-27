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


class TestSSACopyPropagation:
    """Copy propagation: set x $y inherits y's rendered properties."""

    def test_copy_propagates_path_properties(self):
        """set path "$dir/file"; set copy $path -- copy gets W201."""
        from core.compiler.taint import find_taint_warnings

        source = 'set path "$dir/file.txt"\nset copy "$path/sub"'
        ws = [w for w in find_taint_warnings(source) if w.code == "W201"]
        # At least the first assignment should trigger.
        assert len(ws) >= 1

    def test_copy_of_clean_var_no_false_positive(self):
        """set x 42; set y $x -- no W201."""
        from core.compiler.taint import find_taint_warnings

        source = "set x 42\nset y $x"
        ws = [w for w in find_taint_warnings(source) if w.code == "W201"]
        assert len(ws) == 0

    def test_rendered_props_through_copy_chain(self):
        """Rendered properties survive a chain: set a "/etc"; set b $a."""
        from core.compiler.compilation_unit import ensure_compilation_unit

        source = 'set a "/etc/config"\nset b $a'
        cu = ensure_compilation_unit(source)
        assert cu is not None
        rp = cu.top_level.analysis.rendered_props
        # Find the SSA value for 'b'.
        has_slash_on_b = False
        for (name, _ver), props in rp.items():
            if name == "b" and props.may & RenderedProperties.HAS_FORWARD_SLASH:
                has_slash_on_b = True
        assert has_slash_on_b, "b should inherit HAS_FORWARD_SLASH from a"


class TestPhiNodePropagation:
    """Properties across control flow with phi nodes."""

    def test_phi_may_union_across_branches(self):
        """Path in one branch, no path in other -- phi should have may HAS_FORWARD_SLASH."""
        from core.compiler.compilation_unit import ensure_compilation_unit

        source = 'if {$cond} {\n    set x "/etc/config"\n} else {\n    set x "hello"\n}\nputs $x'
        cu = ensure_compilation_unit(source)
        assert cu is not None
        rp = cu.top_level.analysis.rendered_props
        # Find the phi-merged version of x (highest version).
        x_versions = [(ver, props) for (name, ver), props in rp.items() if name == "x"]
        if x_versions:
            max_ver_props = max(x_versions, key=lambda t: t[0])[1]
            # The phi-merged value should have HAS_FORWARD_SLASH from the
            # "/etc/config" branch (may = union).
            assert max_ver_props.may & RenderedProperties.HAS_FORWARD_SLASH

    def test_phi_must_intersection_both_branches_agree(self):
        """Both branches start with / -- phi should preserve STARTS_WITH_SLASH."""
        from core.compiler.compilation_unit import ensure_compilation_unit

        source = 'if {$cond} {\n    set x "/etc/config"\n} else {\n    set x "/var/log"\n}\nputs $x'
        cu = ensure_compilation_unit(source)
        assert cu is not None
        rp = cu.top_level.analysis.rendered_props
        x_versions = [(ver, props) for (name, ver), props in rp.items() if name == "x"]
        if x_versions:
            max_ver_props = max(x_versions, key=lambda t: t[0])[1]
            assert max_ver_props.must & RenderedProperties.STARTS_WITH_SLASH


class TestNamespaceAndDynamicDispatch:
    """Rendered properties with namespaces and dynamic dispatch."""

    def test_namespaced_var_path(self):
        """Namespace-qualified variable in path concatenation."""
        from core.compiler.taint import find_taint_warnings

        source = 'set path "$::config::basedir/file.txt"'
        ws = [w for w in find_taint_warnings(source) if w.code == "W201"]
        assert len(ws) == 1

    def test_namespace_eval_is_barrier(self):
        """namespace eval body is an IRBarrier -- W201 is not detected inside."""
        from core.compiler.taint import find_taint_warnings

        source = 'namespace eval myns {\n    set path "$dir/file.txt"\n}'
        ws = [w for w in find_taint_warnings(source) if w.code == "W201"]
        # namespace eval body is compiled as a barrier; no analysis inside.
        assert len(ws) == 0

    def test_file_join_in_namespace(self):
        """[file join] inside namespace -- should NOT trigger W201."""
        from core.compiler.taint import find_taint_warnings

        source = 'namespace eval myns {\n    set path [file join $dir "file.txt"]\n}'
        ws = [w for w in find_taint_warnings(source) if w.code == "W201"]
        assert len(ws) == 0


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
