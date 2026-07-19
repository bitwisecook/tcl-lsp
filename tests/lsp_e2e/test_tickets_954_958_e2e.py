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

"""End-to-end TP/FP/TN/FN matrix for tickets #954–#958.

Drives the packaged LSP server over JSON-RPC (the same surface VS Code uses).
Each ticket is exercised as a matrix:

- **TP** (true positive) — a reference / token / diagnostic that *should* be
  produced is produced.
- **FP** (false positive) — a lookalike that must *not* be counted (a channel
  call sharing a method name, a built-in math function, a bareword operand, an
  undefined-var read in a normal file) is not.
- **TN** (true negative) — a symbol with genuinely nothing to report yields
  nothing (an uncalled method → 0 references).
- **FN** (false negative) — the original bug: a real reference must not be
  dropped (covered by the TP assertions, which failed before the fix).
"""

from __future__ import annotations

import textwrap

import pytest

from ._lsp_helpers import decode_semantic_tokens, start_lines


@pytest.fixture
def legend(lsp_server):
    return lsp_server.initialize_result["capabilities"]["semanticTokensProvider"]["legend"][
        "tokenTypes"
    ]


def _typed_tokens(lsp_server, legend, uri):
    tokens = decode_semantic_tokens(lsp_server.semantic_tokens(uri))
    for tok in tokens:
        tok["type"] = legend[tok["type"]]
    return tokens


def _codes(diags) -> set[str]:
    return {str(d.get("code")) for d in (diags or [])}


def _method_lens(lsp_server, uri, class_qname, method):
    for lens in lsp_server.code_lens(uri) or []:
        data = lens.get("data") or {}
        if data.get("qname") == class_qname and data.get("method") == method:
            return lsp_server.code_lens_resolve(lens)
    return None


def _proc_lens(lsp_server, uri, qname):
    for lens in lsp_server.code_lens(uri) or []:
        if (lens.get("data") or {}).get("qname") == qname:
            return lsp_server.code_lens_resolve(lens)
    return None


# #956 / #957 — TclOO method references


class TestMethodReferencesMatrix:
    def test_tp_obj_dispatch_is_a_reference(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            oo::class create Bar956 {
                method get {key} { return $key }
            }
            set b [Bar956 new]
            puts [$b get foo]
        """)
        lsp_server.open_ready(uri, src)
        # References on the method definition include the `$b get` call (line 4).
        assert 4 in start_lines(lsp_server.references(uri, 1, 11))

    def test_tp_my_dispatch_is_a_reference(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            oo::class create Bar957 {
                method fetch {key} { return $key }
                method get {key} { return [my fetch $key] }
            }
        """)
        lsp_server.open_ready(uri, src)
        # References on `method fetch` include the `my fetch` call (line 2).
        assert 2 in start_lines(lsp_server.references(uri, 1, 11))

    def test_tp_inherited_call_credits_ancestor(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            oo::class create Base956 {
                method speak {} { return woof }
            }
            oo::class create Dog956 {
                superclass Base956
            }
            set d [Dog956 new]
            $d speak
        """)
        lsp_server.open_ready(uri, src)
        # The inherited `$d speak` call (line 7) is a reference to Base956::speak.
        assert 7 in start_lines(lsp_server.references(uri, 1, 11))

    def test_fp_channel_call_is_not_a_method_reference(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            oo::class create Widget956 {
                method close {} { return done }
            }
            set chan [open /tmp/x w]
            $chan close
        """)
        lsp_server.open_ready(uri, src)
        # `$chan close` is a channel op — the SSA lattice never types `$chan` as
        # an object, so line 4 must not be a reference to Widget956::close.
        assert 4 not in start_lines(lsp_server.references(uri, 1, 11))

    def test_tn_uncalled_method_has_zero_reference_lens(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            oo::class create Lonely956 {
                method unused {} { return 1 }
            }
        """)
        lsp_server.open_ready(uri, src)
        resolved = _method_lens(lsp_server, uri, "::Lonely956", "unused")
        assert resolved is not None
        assert resolved["command"]["title"] == "0 references"

    def test_tp_method_reference_lens_counts_dispatch(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            oo::class create Counter956 {
                method tick {} { return 1 }
            }
            set c [Counter956 new]
            $c tick
            $c tick
        """)
        lsp_server.open_ready(uri, src)
        resolved = _method_lens(lsp_server, uri, "::Counter956", "tick")
        assert resolved is not None
        assert resolved["command"]["title"] == "2 references"


# #958 — expr math-function references to ::tcl::mathfunc procs


class TestMathfuncReferencesMatrix:
    def test_tp_expr_call_references_mathfunc_proc(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            namespace eval ::tcl::mathfunc {
                proc Pi958 {} { return 3.14159 }
            }
            set x [expr {Pi958() / 2.0}]
        """)
        lsp_server.open_ready(uri, src)
        # References on the proc name include the expr call site (line 3).
        assert 3 in start_lines(lsp_server.references(uri, 1, 9))

    def test_fp_bareword_operand_is_not_a_call(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            namespace eval ::tcl::mathfunc {
                proc Pi958c {} { return 3.14 }
            }
            set x abc
            set y [expr {$x eq Pi958c}]
            set z [expr {Pi958c()}]
        """)
        lsp_server.open_ready(uri, src)
        # Only the genuine call `Pi958c()` (line 5) is a reference — the bareword
        # operand `Pi958c` in the string compare (line 4) is not a call.
        lines = start_lines(lsp_server.references(uri, 1, 9, include_declaration=False))
        assert lines == {5}

    def test_tp_reference_lens_counts_expr_calls(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            namespace eval ::tcl::mathfunc {
                proc Tau958 {} { return 6.28 }
            }
            set a [expr {Tau958() + 1}]
            set b [expr {Tau958() + 2}]
        """)
        lsp_server.open_ready(uri, src)
        resolved = _proc_lens(lsp_server, uri, "::tcl::mathfunc::Tau958")
        assert resolved is not None
        assert resolved["command"]["title"] == "2 references"


# #954 — apply lambda body highlighting


class TestApplyHighlightingMatrix:
    def test_tp_body_commands_highlighted(self, lsp_server, uri_factory, legend):
        uri = uri_factory()
        src = "apply {dir {\n    set x [file join $dir a]\n}} $d\n"
        lsp_server.open_ready(uri, src)
        tokens = _typed_tokens(lsp_server, legend, uri)
        line1 = {t["type"] for t in tokens if t["line"] == 1}
        # Body command `set` is a keyword and `$dir` a variable — not one opaque
        # braced string as before the fix.
        assert "keyword" in line1
        assert "variable" in line1
        # The lambda parameter `dir` is a parameter (line 0).
        assert any(t["type"] == "parameter" for t in tokens if t["line"] == 0)

    def test_tn_non_literal_lambda_is_untouched(self, lsp_server, uri_factory, legend):
        uri = uri_factory()
        lsp_server.open_ready(uri, "apply $lambda 1 2\n")
        tokens = _typed_tokens(lsp_server, legend, uri)
        # No crash; the variable receiver is still tokenised.
        assert any(t["type"] == "variable" for t in tokens)


# #955 — read-before-set range precision + pkgIndex suppression


_PKGINDEX_SRC = "package ifneeded foo 1.0 [list source [file join $dir foo.tcl]]\n"


class TestReadBeforeSetMatrix:
    def test_tp_nested_dir_range_points_at_var(self, lsp_server, uri_factory):
        uri = uri_factory()  # not a pkgIndex.tcl name
        diags = lsp_server.open_ready(uri, _PKGINDEX_SRC)
        w210 = [d for d in diags if d.get("code") == "W210"]
        assert len(w210) == 1
        rng = w210[0]["range"]
        line0 = _PKGINDEX_SRC.split("\n")[0]
        # The highlighted span is exactly `$dir`, not the whole statement.
        assert line0[rng["start"]["character"] : rng["end"]["character"]] == "$dir"

    def test_fp_pkgindex_dir_is_suppressed(self, lsp_server):
        # The ambient ``dir`` is provided by the package loader in pkgIndex.tcl.
        uri = "file:///e2e/ticket955_pkg/pkgIndex.tcl"
        diags = lsp_server.open_ready(uri, _PKGINDEX_SRC)
        assert "W210" not in _codes(diags)

    def test_tn_defined_var_has_no_w210(self, lsp_server, uri_factory):
        uri = uri_factory()
        diags = lsp_server.open_ready(uri, "set dir /opt/pkg\nputs [file join $dir a]\n")
        assert "W210" not in _codes(diags)
