"""Completion, end-to-end against the packaged server.

Full-parity port of the request/response cases in ``tests/test_completion.py``.
The text-edit assertions matter on the live surface: an editor applies
``textEdit`` verbatim, so a wrong replace range is exactly the class of
regression the in-process test can mask.

Cases that drive the provider through kwargs with no JSON-RPC surface
(``workspace_procs=`` / ``workspace_command_usage=`` ranking, the
``analysis=`` snapshot redirect, and the ``_var_needs_braces`` /
``split_array_name`` helpers) stay unit-tested in ``tests/test_completion.py``.
The F5 iRules argument/keyword completions live in
``tests/lsp_e2e/test_irules_e2e.py`` (dedicated dialect server).
"""

from __future__ import annotations

import textwrap

from ._lsp_helpers import completion_items, completion_labels


def _labels(lsp_server, uri, line, char):
    return completion_labels(lsp_server.completion(uri, line, char))


def _by_label(lsp_server, uri, line, char):
    return {i["label"]: i for i in completion_items(lsp_server.completion(uri, line, char))}


class TestCommandCompletion:
    def test_empty_line_returns_commands(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "")
        assert {"set", "proc", "puts"} <= set(_labels(lsp_server, uri, 0, 0))

    def test_partial_command(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "pu")
        labels = _labels(lsp_server, uri, 0, 2)
        assert "puts" in labels
        assert "set" not in labels

    def test_no_math_operators(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "")
        labels = _labels(lsp_server, uri, 0, 0)
        assert "+" not in labels
        assert "-" not in labels

    def test_user_proc_in_completions(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc myHelper {x} { return $x }\nmy")
        assert "myHelper" in _labels(lsp_server, uri, 1, 2)

    def test_builtin_command_has_documentation(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "se")
        item = _by_label(lsp_server, uri, 0, 2)["set"]
        doc = item.get("documentation")
        doc_text = doc if isinstance(doc, str) else (doc or {}).get("value", "")
        assert "variable" in doc_text.lower() or "value" in doc_text.lower()


class TestVariableCompletion:
    def test_dollar_triggers_vars(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set greeting hello\nputs $")
        assert "$greeting" in _labels(lsp_server, uri, 1, 6)

    def test_partial_var_name(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set greeting hello\nset goodbye bye\nputs $gre")
        labels = _labels(lsp_server, uri, 2, 9)
        assert "$greeting" in labels
        assert "$goodbye" not in labels

    def test_var_in_proc_scope(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc foo {x} {\n    set local 1\n    puts $\n}\n"
        lsp_server.open_ready(uri, src)
        labels = _labels(lsp_server, uri, 2, 10)
        assert "$x" in labels
        assert "$local" in labels

    def test_namespace_var_completion(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "namespace eval myns {\n    variable nsVar 1\n    puts $\n}\n"
        lsp_server.open_ready(uri, src)
        assert "$nsVar" in _labels(lsp_server, uri, 2, 10)

    def test_dollar_text_edit_replaces_dollar_sign(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set testvar Test\nputs $")
        edit = _by_label(lsp_server, uri, 1, 6)["$testvar"]["textEdit"]
        assert edit["range"]["start"]["character"] == 5
        assert edit["range"]["end"]["character"] == 6
        assert edit["newText"] == "$testvar"

    def test_dollar_text_edit_replaces_partial_var_name(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set greeting hello\nputs $gre")
        edit = _by_label(lsp_server, uri, 1, 9)["$greeting"]["textEdit"]
        assert edit["range"]["start"]["character"] == 5
        assert edit["range"]["end"]["character"] == 9
        assert edit["newText"] == "$greeting"

    def test_dollar_text_edit_brace_form(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set greeting hello\nputs ${gre")
        edit = _by_label(lsp_server, uri, 1, 10)["$greeting"]["textEdit"]
        assert edit["range"]["start"]["character"] == 5
        assert edit["range"]["end"]["character"] == 10
        assert edit["newText"] == "${greeting}"

    def test_dollar_text_edit_brace_form_consumes_existing_close(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set greeting hello\nputs ${gre}")
        edit = _by_label(lsp_server, uri, 1, 10)["$greeting"]["textEdit"]
        assert edit["range"]["start"]["character"] == 5
        assert edit["range"]["end"]["character"] == 11
        assert edit["newText"] == "${greeting}"

    def test_dollar_text_edit_brace_form_empty_with_close(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set greeting hello\nputs ${}")
        edit = _by_label(lsp_server, uri, 1, 7)["$greeting"]["textEdit"]
        assert edit["range"]["start"]["character"] == 5
        assert edit["range"]["end"]["character"] == 8
        assert edit["newText"] == "${greeting}"

    def test_dollar_text_edit_midword_replaces_full_token(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set greeting hello\nputs $greeting")
        edit = _by_label(lsp_server, uri, 1, 9)["$greeting"]["textEdit"]
        assert edit["range"]["start"]["character"] == 5
        assert edit["range"]["end"]["character"] == 14
        assert edit["newText"] == "$greeting"

    def test_dollar_text_edit_midword_brace_form_replaces_full_token(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set greeting hello\nputs ${greeting}")
        edit = _by_label(lsp_server, uri, 1, 10)["$greeting"]["textEdit"]
        assert edit["range"]["start"]["character"] == 5
        assert edit["range"]["end"]["character"] == 16
        assert edit["newText"] == "${greeting}"

    def test_dollar_completion_omits_unsubstitutable_brace_names(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set a_\\}_closebrace 1\nputs $")
        for label in _labels(lsp_server, uri, 1, 6):
            assert "}" not in label

    def test_dollar_text_edit_brace_midword_with_hyphenated_name(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'set "foo-bar" 1\nputs ${foo-bar}')
        by = _by_label(lsp_server, uri, 1, 10)
        assert "$foo-bar" in by
        edit = by["$foo-bar"]["textEdit"]
        assert edit["range"]["start"]["character"] == 5
        assert edit["range"]["end"]["character"] == 15
        assert edit["newText"] == "${foo-bar}"

    def test_dollar_auto_braces_var_name_with_hyphen(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'set "foo-bar" 1\nputs $')
        by = _by_label(lsp_server, uri, 1, 6)
        assert "$foo-bar" in by
        assert by["$foo-bar"]["textEdit"]["newText"] == "${foo-bar}"

    def test_dollar_cross_namespace_offers_qualified_name(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            namespace eval ::other {
                variable baz 3
            }
            namespace eval ::myns {
                puts $
            }
        """)
        lsp_server.open_ready(uri, src)
        by = _by_label(lsp_server, uri, 4, 10)
        assert "$::other::baz" in by
        assert by["$::other::baz"]["textEdit"]["newText"] == "$::other::baz"

    def test_dollar_same_namespace_uses_bare_minimal_form(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "namespace eval ::myns {\n    variable foo 1\n    puts $\n}\n"
        lsp_server.open_ready(uri, src)
        labels = _labels(lsp_server, uri, 2, 10)
        assert "$foo" in labels
        assert "$::myns::foo" not in labels

    def test_dollar_partial_filters_cross_namespace(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            namespace eval ::other {
                variable baz 3
            }
            namespace eval ::myns {
                variable foo 1
                puts $::ot
            }
        """)
        lsp_server.open_ready(uri, src)
        labels = _labels(lsp_server, uri, 5, 14)
        assert "$::other::baz" in labels
        assert "$foo" not in labels

    def test_dollar_completion_offers_dict_for_loop_vars(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc p {} {\n    dict for {k v} {a 1 b 2} {\n        puts $\n    }\n}\n"
        lsp_server.open_ready(uri, src)
        labels = _labels(lsp_server, uri, 2, 14)
        assert "$k" in labels
        assert "$v" in labels

    def test_dollar_completion_offers_dict_with_keys_from_const_literal(
        self, lsp_server, uri_factory
    ):
        uri = uri_factory()
        src = "proc p {} {\n    set d {name alice age 30}\n    dict with d {\n        puts $\n    }\n}\n"
        lsp_server.open_ready(uri, src)
        labels = _labels(lsp_server, uri, 3, 14)
        assert "$name" in labels
        assert "$age" in labels

    def test_dollar_completion_tolerates_cursor_past_eol(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set greeting hello\nputs $")
        assert "$greeting" in _labels(lsp_server, uri, 1, 100)


class TestArrayElementCompletion:
    def test_array_element_completion_offers_known_indices(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc p {} {\n    set arr(name) hello\n    set arr(age) 42\n    puts $arr(\n}\n"
        lsp_server.open_ready(uri, src)
        labels = _labels(lsp_server, uri, 3, 14)
        assert "$arr(name)" in labels
        assert "$arr(age)" in labels

    def test_array_element_completion_filters_by_partial(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc p {} {\n    set arr(name) hello\n    set arr(age) 42\n    puts $arr(na\n}\n"
        lsp_server.open_ready(uri, src)
        assert sorted(_labels(lsp_server, uri, 3, 16)) == ["$arr(name)"]

    def test_array_element_completion_consumes_existing_close_paren(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc p {} {\n    set arr(name) hello\n    puts $arr()\n}\n"
        lsp_server.open_ready(uri, src)
        edit = _by_label(lsp_server, uri, 2, 14)["$arr(name)"]["textEdit"]
        assert edit["range"]["end"]["character"] == 15
        assert edit["newText"] == "$arr(name)"


class TestSubcommandCompletion:
    def test_string_subcommands(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "string ")
        labels = _labels(lsp_server, uri, 0, 7)
        assert {"length", "match", "tolower"} <= set(labels)

    def test_partial_subcommand(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "string to")
        labels = _labels(lsp_server, uri, 0, 9)
        assert {"tolower", "toupper", "totitle"} <= set(labels)
        assert "length" not in labels

    def test_namespace_subcommands(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "namespace ")
        labels = _labels(lsp_server, uri, 0, 10)
        assert {"eval", "export"} <= set(labels)


class TestSwitchCompletion:
    def test_regexp_switches(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "regexp -")
        labels = _labels(lsp_server, uri, 0, 8)
        assert {"-nocase", "-all"} <= set(labels)

    def test_partial_switch(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "lsort -no")
        assert "-nocase" in _labels(lsp_server, uri, 0, 9)

    def test_socket_switches(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "socket -")
        labels = _labels(lsp_server, uri, 0, 8)
        assert {"-server", "-myaddr"} <= set(labels)

    def test_switch_completion_ignores_semicolon_in_quoted_arg(self, lsp_server, uri_factory):
        uri = uri_factory()
        source = 'socket "a;b" -'
        lsp_server.open_ready(uri, source)
        assert "-server" in _labels(lsp_server, uri, 0, len(source))

    def test_switch_text_edit_replaces_partial_dash_prefix(self, lsp_server, uri_factory):
        uri = uri_factory()
        source = "lsort -no"
        lsp_server.open_ready(uri, source)
        edit = _by_label(lsp_server, uri, 0, len(source))["-nocase"]["textEdit"]
        assert edit["range"]["start"]["character"] == 6
        assert edit["range"]["end"]["character"] == 9
        assert edit["newText"] == "-nocase"

    def test_switch_text_edit_bare_dash(self, lsp_server, uri_factory):
        uri = uri_factory()
        source = "regexp -"
        lsp_server.open_ready(uri, source)
        edit = _by_label(lsp_server, uri, 0, len(source))["-nocase"]["textEdit"]
        assert edit["range"]["start"]["character"] == 7
        assert edit["range"]["end"]["character"] == 8
        assert edit["newText"] == "-nocase"

    def test_switch_has_documentation(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "socket -")
        item = _by_label(lsp_server, uri, 0, 8)["-server"]
        doc = item.get("documentation")
        doc_str = doc if isinstance(doc, str) else (doc or {}).get("value", "")
        assert len(doc_str) > 0

    def test_switch_text_edit_with_single_char_partial(self, lsp_server, uri_factory):
        uri = uri_factory()
        source = "regexp -n"
        lsp_server.open_ready(uri, source)
        edit = _by_label(lsp_server, uri, 0, len(source))["-nocase"]["textEdit"]
        assert edit["range"]["start"]["character"] == 7
        assert edit["range"]["end"]["character"] == 9
        assert edit["newText"] == "-nocase"

    def test_switch_text_edit_with_longer_partial(self, lsp_server, uri_factory):
        uri = uri_factory()
        source = "lsort -noc"
        lsp_server.open_ready(uri, source)
        edit = _by_label(lsp_server, uri, 0, len(source))["-nocase"]["textEdit"]
        assert edit["range"]["start"]["character"] == 6
        assert edit["range"]["end"]["character"] == 10
        assert edit["newText"] == "-nocase"


class TestScopeBindingCompletion:
    def test_dollar_global_var_from_namespace_offers_qualified(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "set ::globalvar 9\nnamespace eval ::myns {\n    puts $\n}\n"
        lsp_server.open_ready(uri, src)
        assert "$::globalvar" in _labels(lsp_server, uri, 2, 10)

    def test_dollar_completion_offers_global_local_alias(self, lsp_server, uri_factory):
        for decl in ("global myglobal", "global ::myglobal"):
            uri = uri_factory()
            src = f"set ::myglobal 9\nproc p {{}} {{\n    {decl}\n    puts $\n}}\n"
            lsp_server.open_ready(uri, src)
            labels = set(_labels(lsp_server, uri, 3, 10))
            assert "$myglobal" in labels
            assert "$::myglobal" in labels

    def test_dollar_completion_offers_upvar_alias(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc inner {var} {\n    upvar 1 $var alias\n    puts $\n}\n"
        lsp_server.open_ready(uri, src)
        assert "$alias" in _labels(lsp_server, uri, 2, 10)

    def test_dollar_completion_offers_namespace_upvar_alias(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = (
            "namespace eval ::ns { variable foo 1 }\n"
            "proc p {} {\n    namespace upvar ::ns foo localfoo\n    puts $\n}\n"
        )
        lsp_server.open_ready(uri, src)
        assert "$localfoo" in _labels(lsp_server, uri, 3, 10)

    def test_dollar_completion_offers_try_on_error_bindings(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = (
            "proc p {} {\n    try {\n        error boom\n"
            "    } on error {try_msg try_opts} {\n        puts $\n    }\n}\n"
        )
        lsp_server.open_ready(uri, src)
        labels = _labels(lsp_server, uri, 4, 18)
        assert "$try_msg" in labels
        assert "$try_opts" in labels

    def test_dollar_completion_offers_dict_update_aliases(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = (
            "proc p {} {\n    set d {a 1 b 2}\n"
            "    dict update d a varA b varB {\n        puts $\n    }\n}\n"
        )
        lsp_server.open_ready(uri, src)
        labels = _labels(lsp_server, uri, 3, 14)
        assert "$varA" in labels
        assert "$varB" in labels

    def test_dollar_completion_uplevel_zero_uses_global_scope(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = (
            "set ::globalvar 9\nproc p {} {\n    set local_var 1\n"
            "    uplevel #0 {\n        set inside_var 99\n        puts $\n    }\n}\n"
        )
        lsp_server.open_ready(uri, src)
        labels = _labels(lsp_server, uri, 5, 18)
        assert "$::globalvar" in labels
        assert "$inside_var" in labels
        assert "$local_var" not in labels

    def test_dollar_completion_uplevel_one_keeps_proc_scope(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc p {} {\n    set local_var 1\n    uplevel 1 {\n        puts $\n    }\n}\n"
        lsp_server.open_ready(uri, src)
        assert "$local_var" in _labels(lsp_server, uri, 3, 14)


class TestArrayReadOnlyIndices:
    def test_array_element_completion_picks_up_read_only_indices(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc p {} {\n    set arr(name) hello\n    puts $arr(role)\n    puts $arr(\n}\n"
        lsp_server.open_ready(uri, src)
        labels = _labels(lsp_server, uri, 3, 14)
        assert "$arr(name)" in labels
        assert "$arr(role)" in labels
