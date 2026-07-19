# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Regression tests for tickets #954–#958 — TclOO/apply/mathfunc name resolution.

Each class targets one ticket:

- #954: commands inside an ``apply`` lambda body are semantically highlighted.
- #955: the W210 read-before-set range points at the offending ``$var`` even
  when it is nested in a command substitution (and pkgIndex ``$dir`` stays
  suppressed).
- #956: ``$obj method`` dispatch counts as a reference to the method.
- #957: ``my method`` self-dispatch counts as a reference to the method.
- #958: an ``expr`` function call ``Foo()`` counts as a reference to the proc
  ``::tcl::mathfunc::Foo``.
"""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser import analyse
from server._lsp_conv import to_lsp_location
from server.features import SEMANTIC_TOKEN_TYPES, semantic_tokens_full
from server.features.code_lens import get_code_lenses, resolve_code_lens
from server.features.diagnostics import get_diagnostics
from server.features.references import (
    find_method_call_sites,
    find_proc_call_sites,
    get_references,
)

TEST_URI = "file:///test.tcl"


def _decode_tokens(data: list[int]) -> list[dict]:
    """Decode the flat 5-int semantic-token stream into readable dicts."""
    tokens = []
    line = char = 0
    for i in range(0, len(data), 5):
        delta_line, delta_char, length, type_idx, _mods = data[i : i + 5]
        if delta_line > 0:
            line += delta_line
            char = delta_char
        else:
            char += delta_char
        tokens.append(
            {"line": line, "char": char, "length": length, "type": SEMANTIC_TOKEN_TYPES[type_idx]}
        )
    return tokens


class _FakeWorkspaceIndex:
    def proc_usage_counts(self) -> dict[str, int]:
        return {}


# #954 -- apply lambda body highlighting


class TestApplyBodyHighlighting:
    def test_body_commands_are_highlighted(self):
        source = textwrap.dedent("""\
            apply {dir {
                set x [file join $dir a]
            }} $d
        """)
        tokens = _decode_tokens(semantic_tokens_full(source))
        by_line: dict[int, list[dict]] = {}
        for tok in tokens:
            by_line.setdefault(tok["line"], []).append(tok)

        # The lambda parameter is emitted as a parameter declaration.
        assert any(t["type"] == "parameter" for t in by_line.get(0, []))
        # The body command ``set`` is a keyword, and ``$dir`` a variable —
        # before the fix the whole body line was a single opaque string token.
        line1_types = {t["type"] for t in by_line.get(1, [])}
        assert "keyword" in line1_types
        assert "variable" in line1_types
        # No single string token spans the whole body line.
        assert not any(t["type"] == "string" and t["length"] > 20 for t in by_line.get(1, []))

    def test_braced_param_list_with_defaults(self):
        source = "set f [apply {{a {b 5} args} {\n    puts $a\n}} 1]\n"
        tokens = _decode_tokens(semantic_tokens_full(source))
        params = [t for t in tokens if t["type"] == "parameter"]
        # a, b, and args are all emitted as parameters (defaults dropped).
        assert len(params) == 3

    def test_non_literal_lambda_is_left_untouched(self):
        # ``apply $lambda`` cannot be split; must not crash and emits the var.
        tokens = _decode_tokens(semantic_tokens_full("apply $lambda 1 2\n"))
        assert any(t["type"] == "variable" for t in tokens)


# #955 -- read-before-set range precision + pkgIndex suppression


_PKGINDEX_SRC = "package ifneeded foo 1.0 [list source [file join $dir foo.tcl]]\n"


class TestReadBeforeSetRange:
    def test_nested_dir_range_points_at_var_not_statement(self):
        diags = [
            d
            for d in get_diagnostics(_PKGINDEX_SRC, uri="file:///proj/other.tcl")
            if d.code == "W210"
        ]
        assert len(diags) == 1
        r = diags[0].range
        line0 = _PKGINDEX_SRC.split("\n")[0]
        # The highlighted span is exactly the ``$dir`` read, not the whole
        # ``package ifneeded ...`` statement.
        assert r.start.line == 0 and r.end.line == 0
        assert line0[r.start.character : r.end.character] == "$dir"

    def test_apply_wrapped_dir_range_excludes_body(self):
        source = (
            "package ifneeded pkg 1.0 [list apply {dir {\n"
            "    source [file join $dir a.tcl]\n"
            "}} $dir]\n"
        )
        diags = [
            d for d in get_diagnostics(source, uri="file:///proj/other.tcl") if d.code == "W210"
        ]
        assert len(diags) == 1
        r = diags[0].range
        # The read that fires is the trailing ``$dir`` on the last line, not the
        # lambda body — the range must land there.
        assert r.start.line == 2

    def test_pkgindex_dir_still_suppressed(self):
        codes = [d.code for d in get_diagnostics(_PKGINDEX_SRC, uri="file:///pkg/pkgIndex.tcl")]
        assert "W210" not in codes


# #956 / #957 -- TclOO method references


_OO_OBJ_SRC = textwrap.dedent("""\
    oo::class create Bar {
        variable _options
        constructor {args} { set _options $args }
        method get {key} { return [dict get $_options $key] }
    }
    set b [Bar new]
    puts [$b get foo]
""")

_OO_MY_SRC = textwrap.dedent("""\
    oo::class create Bar {
        variable _options
        constructor {args} { set _options $args }
        method getOptions {key} { return [dict get $_options $key] }
        method get {key} { return [my getOptions $key] }
    }
    set b [Bar new]
""")


class TestMethodReferences:
    def test_obj_method_call_is_captured_with_class(self):
        analysis = analyse(_OO_OBJ_SRC)
        calls = [mi for mi in analysis.method_invocations if mi.method_name == "get"]
        assert len(calls) == 1
        assert calls[0].class_name == "::Bar"
        assert calls[0].receiver == "obj"

    def test_obj_method_find_references(self):
        analysis = analyse(_OO_OBJ_SRC)
        sites = find_method_call_sites("::Bar", "get", analysis)
        assert len(sites) == 1
        # get_references from the method definition returns decl + the call.
        method = analysis.all_classes["::Bar"].methods["get"]
        refs = get_references(
            _OO_OBJ_SRC,
            TEST_URI,
            method.name_range.start.line,
            method.name_range.start.character,
            analysis,
        )
        assert len(refs) == 2

    def test_my_method_call_is_captured_with_enclosing_class(self):
        analysis = analyse(_OO_MY_SRC)
        calls = [mi for mi in analysis.method_invocations if mi.method_name == "getOptions"]
        assert len(calls) == 1
        assert calls[0].class_name == "::Bar"
        assert calls[0].receiver == "my"

    def test_my_method_find_references(self):
        analysis = analyse(_OO_MY_SRC)
        assert len(find_method_call_sites("::Bar", "getOptions", analysis)) == 1
        # ``get`` is only defined, never called -> no call sites.
        assert len(find_method_call_sites("::Bar", "get", analysis)) == 0

    def test_inherited_call_is_credited_to_the_defining_ancestor(self):
        # A call through a subclass that inherits a method is attributed to the
        # class that actually defines it — the SSA type of ``$d`` is ``Dog`` and
        # the hierarchy resolves ``speak`` to its provider ``Base``.
        src = textwrap.dedent("""\
            oo::class create Base { method speak {} { return woof } }
            oo::class create Dog { superclass Base }
            set d [Dog new]
            $d speak
        """)
        analysis = analyse(src)
        assert len(find_method_call_sites("::Base", "speak", analysis)) == 1
        # Dog does not define speak itself, so it has no own reference.
        assert find_method_call_sites("::Dog", "speak", analysis) == []

    def test_untyped_receiver_is_not_a_method_reference(self):
        # A channel/ensemble call on an untyped variable must not be miscounted
        # as a method reference, even when the second word matches a method name
        # (``$chan close`` is a channel op, not ``Widget close``).  The SSA
        # OBJECT lattice never types ``$chan`` as an object, so no guard is
        # needed — the receiver simply resolves to no class.
        src = textwrap.dedent("""\
            oo::class create Widget { method close {} { return done } }
            set chan [open /tmp/x w]
            $chan close
        """)
        analysis = analyse(src)
        assert find_method_call_sites("::Widget", "close", analysis) == []

    def test_method_reference_code_lens_count(self):
        analysis = analyse(_OO_OBJ_SRC)
        lenses = get_code_lenses(_OO_OBJ_SRC, TEST_URI, analysis)
        method_lenses = [
            lens
            for lens in lenses
            if isinstance(lens.data, dict) and lens.data.get("kind") == "method_ref_count"
        ]
        assert method_lenses, "expected a reference lens above the method"

        def find_method_refs(uri: str, class_qname: str, method_name: str):
            return [
                to_lsp_location(uri, r)
                for r in find_method_call_sites(class_qname, method_name, analysis)
            ]

        get_lens = next(
            lens
            for lens in method_lenses
            if isinstance(lens.data, dict) and lens.data.get("method") == "get"
        )
        resolved = resolve_code_lens(get_lens, _FakeWorkspaceIndex(), None, find_method_refs)
        assert resolved.command is not None
        assert resolved.command.title == "1 reference"


# #958 -- expr math-function references to ::tcl::mathfunc procs


_MATHFUNC_SRC = textwrap.dedent("""\
    namespace eval ::tcl::mathfunc {
        proc Pi {} { return [expr {acos(-1)}] }
        proc deg2rad {deg} { return [expr {$deg * Pi() / 180.0}] }
    }
    set pi2 [expr {Pi() / 2.0}]
    set rad [expr {deg2rad(45)}]
""")


class TestMathfuncExprReferences:
    def test_expr_calls_reference_mathfunc_proc(self):
        analysis = analyse(_MATHFUNC_SRC)
        # Pi() is called inside deg2rad's body and at the top level.
        sites = find_proc_call_sites("Pi", "::tcl::mathfunc::Pi", analysis)
        assert len(sites) == 2
        # deg2rad() is called once at the top level.
        assert len(find_proc_call_sites("deg2rad", "::tcl::mathfunc::deg2rad", analysis)) == 1

    def test_get_references_includes_expr_call_sites(self):
        analysis = analyse(_MATHFUNC_SRC)
        pi = analysis.all_procs["::tcl::mathfunc::Pi"]
        refs = get_references(
            _MATHFUNC_SRC,
            TEST_URI,
            pi.name_range.start.line,
            pi.name_range.start.character,
            analysis,
        )
        # Declaration + two call sites.
        assert len(refs) == 3

    def test_builtin_math_function_is_not_a_proc_reference(self):
        analysis = analyse(_MATHFUNC_SRC)
        # ``acos`` is a built-in with no user proc, so it must not appear as an
        # invocation naming a mathfunc command.
        assert not any(
            inv.resolved_qualified_name == "::tcl::mathfunc::acos"
            for inv in analysis.command_invocations
        )

    def test_bareword_operand_is_not_a_call(self):
        # ``foo`` after ``eq`` lexes as a FUNCTION token but is not a call, so it
        # must not be recorded as a reference even if such a proc existed.
        src = textwrap.dedent("""\
            proc ::tcl::mathfunc::foo {} { return 1 }
            set x abc
            set y [expr {$x eq foo}]
        """)
        analysis = analyse(src)
        assert find_proc_call_sites("foo", "::tcl::mathfunc::foo", analysis) == []
