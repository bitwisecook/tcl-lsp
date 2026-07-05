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

"""Tests for the Godbolt-style web compiler explorer."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

pytest.importorskip("flask", reason="flask not installed")

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

try:
    from tooling.explorer.web import app  # noqa: E402
except ModuleNotFoundError:
    pytest.skip("explorer.web module not available", allow_module_level=True)

# Fixtures


@pytest.fixture()
def client():
    app.config["TESTING"] = True
    with app.test_client() as c:
        yield c


# Index / HTML page


class TestIndex:
    def test_serves_html(self, client):
        resp = client.get("/")
        assert resp.status_code == 200
        assert resp.content_type.startswith("text/html")

    def test_html_contains_title(self, client):
        resp = client.get("/")
        assert b"Tcl Compiler Explorer" in resp.data

    def test_html_contains_editor_textarea(self, client):
        resp = client.get("/")
        assert b'id="source"' in resp.data

    def test_html_contains_tab_bar(self, client):
        resp = client.get("/")
        assert b"tab-bar" in resp.data
        assert b"data-tab" in resp.data


# /api/compile — error handling


class TestCompileErrors:
    def test_empty_source_returns_400(self, client):
        resp = client.post("/api/compile", json={"source": "   "})
        assert resp.status_code == 400
        data = resp.get_json()
        assert "error" in data

    def test_missing_source_returns_400(self, client):
        resp = client.post("/api/compile", json={})
        assert resp.status_code == 400


# /api/compile — successful compilation


def _compile(client, source: str) -> dict:
    resp = client.post("/api/compile", json={"source": source})
    assert resp.status_code == 200
    return resp.get_json()


class TestCompileBasic:
    def test_simple_set(self, client):
        data = _compile(client, "set x 1")
        assert "ir" in data
        assert "cfgPreSsa" in data
        assert "cfgPostSsa" in data
        assert "interprocedural" in data
        assert "optimisations" in data
        assert "shimmer" in data
        assert "annotations" in data
        assert "stats" in data

    def test_stats_counts(self, client):
        data = _compile(client, "set x 1")
        stats = data["stats"]
        assert stats["procedures"] == 0
        assert stats["functions"] >= 1
        assert stats["blocks"] >= 1

    def test_response_is_json(self, client):
        resp = client.post("/api/compile", json={"source": "set x 1"})
        assert resp.content_type == "application/json"


# Green tree / loops / intervals / bounds — views brought over from the CLI.


class TestExtraViews:
    def test_keys_present(self, client):
        data = _compile(client, "set x 1")
        for key in ("greentree", "loops", "intervals", "bounds"):
            assert key in data

    def test_greentree_tokens_carry_detail(self, client):
        data = _compile(client, "set x 1")
        gt = data["greentree"]
        assert gt["kind"] == "ROOT"
        assert gt["tokens"]
        tok = gt["tokens"][0]
        for key in ("type", "text", "startOffset", "endOffset", "startLine", "startCol"):
            assert key in tok

    def test_greentree_opaque_token_has_child_region(self, client):
        data = _compile(client, "proc f {} { set y 2 }")

        # Find any token that descended into a child region (the braced body).
        def has_child(node):
            for t in node["tokens"]:
                if "child" in t:
                    return True
                # children only hang off opaque tokens, no deeper walk needed
            return False

        assert has_child(data["greentree"]) or any(
            "child" in t for t in data["greentree"]["tokens"]
        )

    def test_cst_segments_keys_present(self, client):
        data = _compile(client, "set x 1")
        assert "cst" in data
        assert "segments" in data

    def test_cst_structural_shape(self, client):
        data = _compile(client, "proc f {} { set y 2 }")
        cst = data["cst"]
        assert cst["kind"] == "DOCUMENT"
        cmd = cst["children"][0]
        assert cmd["kind"] == "COMMAND"
        assert "proc" in cmd["label"]
        # the proc body word descends into a child document (terminated)
        body_word = cmd["children"][-1]
        assert body_word["kind"] == "WORD"
        assert "braced" in body_word["tags"]
        body_tok = body_word["children"][0]
        assert body_tok["child"]["kind"] == "DOCUMENT"
        assert body_tok["terminated"] is True

    def test_cst_marks_unterminated_recovered(self, client):
        data = _compile(client, "set y [foo {bar")

        def find_recovered(node):
            if node.get("terminated") is False:
                return True
            for c in node.get("children", []):
                if find_recovered(c):
                    return True
            child = node.get("child")
            return bool(child and find_recovered(child))

        assert find_recovered(data["cst"])

    def test_segments_words_and_flags(self, client):
        data = _compile(client, 'foo {*}$args "hi $x" {}')
        segs = data["segments"]
        assert len(segs) == 1
        seg = segs[0]
        assert seg["name"] == "foo"
        flags = {w["text"]: w for w in seg["words"]}
        assert flags["${args}"]["expand"] is True
        assert flags["hi ${x}"]["quoted"] is True
        assert any(w["braced"] for w in seg["words"])  # the {} word

    def test_segments_preceding_comment(self, client):
        data = _compile(client, "# hello\nputs hi")
        seg = data["segments"][0]
        assert seg["precedingComment"] == "hello"

    def test_loops_reports_natural_loop(self, client):
        data = _compile(client, "for {set i 0} {$i < 5} {incr i} { puts $i }")
        loops = [lp for f in data["loops"] for lp in f["loops"]]
        assert loops
        assert "header" in loops[0]
        assert "blockCount" in loops[0]

    def test_intervals_bounded_range(self, client):
        data = _compile(client, "proc f {} { set m 8 ; return [lindex {a} $m] }")
        entries = [e for f in data["intervals"] for e in f["entries"]]
        assert any(e["lo"] is not None or e["hi"] is not None for e in entries)

    def test_bounds_divide_by_zero(self, client):
        data = _compile(client, "proc f {} { return [expr {1 / 0}] }")
        divzero = [d for f in data["bounds"] for d in f["divzero"]]
        assert any(d["code"] == "W233" for d in divzero)


class TestOptLensData:
    SRC = "set a 1\nset b [expr {$a + 2}]\nputs $b"

    def test_optimised_keys_present_when_rewritten(self, client):
        data = _compile(client, self.SRC)
        assert data["irOptimised"] is not None
        assert data["cfgPreSsaOptimised"] is not None
        assert data["cfgPostSsaOptimised"] is not None
        summaries = [n["summary"] for n in data["irOptimised"]["topLevel"]]
        assert any("puts 3" in s for s in summaries)

    def test_optimised_keys_null_when_unchanged(self, client):
        data = _compile(client, "puts hi")
        assert data["irOptimised"] is None
        assert data["cfgPreSsaOptimised"] is None
        assert data["cfgPostSsaOptimised"] is None


# IR serialisation


class TestIR:
    def test_top_level_statements(self, client):
        data = _compile(client, "set x 1\nputs $x")
        ir = data["ir"]
        # Both top-level statements (set, puts) are present.
        assert len(ir["topLevel"]) == 2

    def test_ir_node_shape(self, client):
        data = _compile(client, "set x 1")
        node = data["ir"]["topLevel"][0]
        assert "kind" in node
        assert "summary" in node
        assert "colorClass" in node
        assert "range" in node

    def test_ir_range_fields(self, client):
        data = _compile(client, "set x 1")
        r = data["ir"]["topLevel"][0]["range"]
        for key in ("startLine", "startCol", "startOffset", "endLine", "endCol", "endOffset"):
            assert key in r
            assert isinstance(r[key], int)

    def test_procedure_ir(self, client):
        data = _compile(client, "proc greet {name} { puts $name }")
        procs = data["ir"]["procedures"]
        assert "::greet" in procs
        proc = procs["::greet"]
        assert proc["params"] == ["name"]
        assert len(proc["body"]) >= 1

    def test_if_ir_has_children(self, client):
        data = _compile(client, "if {1} { set a 1 } else { set a 2 }")
        top = data["ir"]["topLevel"]
        if_node = [n for n in top if n["kind"] == "IRIf"]
        assert if_node
        assert "children" in if_node[0]
        assert len(if_node[0]["children"]) >= 2  # clause + else

    def test_for_ir_has_children(self, client):
        data = _compile(client, "for {set i 0} {$i < 5} {incr i} { puts $i }")
        top = data["ir"]["topLevel"]
        for_node = [n for n in top if n["kind"] == "IRFor"]
        assert for_node
        children = for_node[0]["children"]
        labels = [c["label"] for c in children]
        assert "init" in labels
        assert "body" in labels

    def test_switch_ir_has_arms(self, client):
        data = _compile(client, "switch -- $x { a { puts a } b { puts b } }")
        top = data["ir"]["topLevel"]
        sw = [n for n in top if n["kind"] == "IRSwitch"]
        assert sw
        assert "children" in sw[0]
        assert len(sw[0]["children"]) >= 2


# CFG (pre-SSA)


class TestCFGPreSSA:
    def test_top_level_function_present(self, client):
        data = _compile(client, "set x 1")
        assert len(data["cfgPreSsa"]) >= 1
        top = data["cfgPreSsa"][0]
        assert top["name"] == "::top"
        assert top["blockCount"] >= 1

    def test_block_shape(self, client):
        data = _compile(client, "set x 1")
        block = data["cfgPreSsa"][0]["blocks"][0]
        assert "name" in block
        assert "isEntry" in block
        assert "statements" in block
        assert "terminator" in block

    def test_proc_creates_additional_function(self, client):
        data = _compile(client, "proc f {} { return 1 }")
        names = [f["name"] for f in data["cfgPreSsa"]]
        assert "::top" in names
        assert "::f" in names


# CFG (post-SSA) + analysis


class TestCFGPostSSA:
    def test_ssa_info_present(self, client):
        data = _compile(client, "proc f {x} { set y $x\nreturn $y }")
        func = [f for f in data["cfgPostSsa"] if f["name"] == "::f"][0]
        block = func["blocks"][0]
        has_uses_or_defs = any(s.get("uses") or s.get("defs") for s in block["statements"])
        assert has_uses_or_defs

    def test_analysis_section(self, client):
        data = _compile(client, "proc f {x} { return $x }")
        func = [f for f in data["cfgPostSsa"] if f["name"] == "::f"][0]
        analysis = func["analysis"]
        assert "constantBranches" in analysis
        assert "deadStores" in analysis
        assert "unreachableBlocks" in analysis
        assert "inferredTypes" in analysis

    def test_phi_nodes_on_branch(self, client):
        data = _compile(client, "if {$cond} {set a 1} else {set a 2}\nset b $a")
        top = [f for f in data["cfgPostSsa"] if f["name"] == "::top"][0]
        all_phis = []
        for block in top["blocks"]:
            all_phis.extend(block["phis"])
        phi_names = [p["name"] for p in all_phis]
        assert "a" in phi_names

    def test_dead_store_detected(self, client):
        data = _compile(client, "proc f {} { set a 1\nset a 2\nreturn $a }")
        func = [f for f in data["cfgPostSsa"] if f["name"] == "::f"][0]
        assert len(func["analysis"]["deadStores"]) >= 1
        dead = func["analysis"]["deadStores"][0]
        assert dead["variable"] == "a"


# Interprocedural analysis


class TestInterprocedural:
    def test_proc_summary(self, client):
        data = _compile(client, "proc double {x} { expr {$x * 2} }")
        procs = data["interprocedural"]
        assert len(procs) == 1
        p = procs[0]
        assert p["name"] == "::double"
        assert "arity" in p
        assert "pure" in p
        assert "returnShape" in p

    def test_no_procs_empty_list(self, client):
        data = _compile(client, "puts hello")
        assert data["interprocedural"] == []

    def test_calls_tracked(self, client):
        data = _compile(client, "proc a {} { b }\nproc b {} { return 1 }")
        a = [p for p in data["interprocedural"] if p["name"] == "::a"][0]
        assert any("b" in c for c in a["calls"])


# Optimisations


class TestOptimisations:
    def test_constant_fold_detected(self, client):
        data = _compile(client, "set a 1\nset b [expr {$a + 2}]")
        assert len(data["optimisations"]) >= 1
        opt = data["optimisations"][0]
        assert "code" in opt
        assert "message" in opt
        assert "range" in opt
        assert "replacement" in opt

    def test_no_optimisations_for_dynamic(self, client):
        data = _compile(client, "puts [gets stdin]")
        assert data["optimisations"] == []

    def test_optimised_source_present(self, client):
        data = _compile(client, "set a 1\nset b [expr {$a + 2}]")
        # Constant propagation + folding collapses the pair to ``set b 3``.
        assert data["optimisedSource"] == "set b 3"

    def test_optimised_source_none_when_unchanged(self, client):
        data = _compile(client, "puts hello")
        assert data["optimisedSource"] is None


# Shimmer warnings


class TestShimmer:
    def test_shimmer_warning_shape(self, client):
        # The previous snippet produced no shimmer at all, so the shape
        # assertions never ran. Use a string→list intrep change that does
        # raise an S100 shimmer.
        data = _compile(
            client,
            "set myList {a b c d}\nlindex $myList 1\nputs $myList",
        )
        assert data["shimmer"], data
        w = data["shimmer"][0]
        assert w["code"] == "S100"
        assert "message" in w
        assert "range" in w

    def test_no_shimmer_for_safe_code(self, client):
        data = _compile(client, "set x 1\nputs $x")
        assert data["shimmer"] == []


# Annotations


class TestAnnotations:
    def test_annotations_sorted_by_offset(self, client):
        data = _compile(client, "set a 1\nset b [expr {$a + 2}]\nset c [expr {$b + 3}]")
        offsets = [a["range"]["startOffset"] for a in data["annotations"]]
        assert offsets == sorted(offsets)

    def test_annotation_has_kind(self, client):
        data = _compile(client, "set a 1\nset b [expr {$a + 2}]")
        # This snippet is optimised, so annotations must be present and each
        # must carry the editor-facing fields.
        assert data["annotations"], data
        ann = data["annotations"][0]
        assert "kind" in ann
        assert "label" in ann
        assert "range" in ann
