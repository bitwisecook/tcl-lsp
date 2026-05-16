"""Tests for the stub → signature overlay wiring.

Unit-level coverage of :func:`stub_to_command_sig` and
:func:`stub_signature_scope` from
:mod:`core.commands.registry.runtime`, plus end-to-end coverage that
walks all the way down to the call-graph and unused-proc analyser.
"""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.semantic_graph import build_call_graph
from core.analysis.semantic_model import (
    Range,
    SourcePosition,
    StubArgDef,
    StubCommandDef,
)
from core.commands.registry.runtime import (
    SIGNATURES,
    arg_indices_for_role,
    body_arg_indices,
    resolve_arg_role_map,
    signatures_from_stubs,
    stub_signature_scope,
    stub_to_command_sig,
)
from core.commands.registry.signatures import ArgRole

_ZERO = Range(
    start=SourcePosition(line=0, character=0, offset=0),
    end=SourcePosition(line=0, character=0, offset=0),
)


from typing import Any  # noqa: E402


def _stub(name: str, *args: tuple[str, str, bool], **flags: Any) -> StubCommandDef:
    return StubCommandDef(
        name=name,
        args=tuple(StubArgDef(name=n, role=r, optional=o) for n, r, o in args),
        range=_ZERO,
        **flags,
    )


class TestStubToCommandSig:
    def test_body_role_maps_to_argrole_body(self):
        sig = stub_to_command_sig(
            _stub("my_eval", ("sql", "value", False), ("script", "body", False))
        )
        assert sig.arg_roles == {1: frozenset({ArgRole.BODY})}
        assert sig.arity.min == 2
        assert sig.arity.max == 2

    def test_multiple_role_args(self):
        sig = stub_to_command_sig(
            _stub(
                "complex",
                ("name", "name", False),
                ("out", "var", False),
                ("matched", "var_read", False),
                ("re", "pattern", False),
                ("ch", "channel", False),
                ("body", "body", False),
                ("expr_arg", "expr", False),
            )
        )
        assert sig.arg_roles[0] == frozenset({ArgRole.NAME})
        assert sig.arg_roles[1] == frozenset({ArgRole.VAR_WRITE})
        assert sig.arg_roles[2] == frozenset({ArgRole.VAR_READ})
        assert sig.arg_roles[3] == frozenset({ArgRole.PATTERN})
        assert sig.arg_roles[4] == frozenset({ArgRole.CHANNEL})
        assert sig.arg_roles[5] == frozenset({ArgRole.BODY})
        assert sig.arg_roles[6] == frozenset({ArgRole.EXPR})

    def test_unknown_role_value_is_unannotated(self):
        sig = stub_to_command_sig(_stub("plain", ("x", "value", False)))
        # "value" is the default — should not produce a role entry.
        assert sig.arg_roles == {}

    def test_optional_args_lower_min_arity(self):
        sig = stub_to_command_sig(
            _stub(
                "optional",
                ("sql", "value", False),
                ("script", "body", True),
            )
        )
        assert sig.arity.min == 1
        assert sig.arity.max == 2

    def test_args_tail_is_variadic_with_zero_or_more_extras(self):
        from core.commands.registry.signatures import Arity

        sig = stub_to_command_sig(
            _stub(
                "variadic",
                ("first", "value", False),
                ("args", "value", False),
            )
        )
        # ``args`` is Tcl's variadic tail — accepts zero or more
        # arguments and does NOT contribute to the minimum arity.
        # Only ``first`` is required.
        assert sig.arity.min == 1
        assert sig.arity.max == Arity.ANY

    def test_args_only_is_pure_variadic(self):
        from core.commands.registry.signatures import Arity

        sig = stub_to_command_sig(_stub("anything", ("args", "value", False)))
        assert sig.arity.min == 0
        assert sig.arity.max == Arity.ANY

    def test_signatures_from_stubs_strips_leading_namespace(self):
        # ``sqlite::eval`` and ``::sqlite::eval`` should both end up in
        # the overlay under the same key so call-site name forms hit it.
        overlay = signatures_from_stubs([_stub("::ns::foo", ("body", "body", False))])
        assert "ns::foo" in overlay


class TestStubSignatureScope:
    def test_overlay_visible_inside_scope(self):
        stub = _stub("scoped_cmd", ("sql", "value", False), ("script", "body", False))
        assert "scoped_cmd" not in SIGNATURES
        with stub_signature_scope([stub]):
            assert "scoped_cmd" in SIGNATURES
            assert arg_indices_for_role("scoped_cmd", ["x", "y"], ArgRole.BODY) == {1}
        assert "scoped_cmd" not in SIGNATURES

    def test_overlay_restored_on_exception(self):
        stub = _stub("ex_cmd", ("script", "body", False))
        try:
            with stub_signature_scope([stub]):
                assert "ex_cmd" in SIGNATURES
                raise RuntimeError("boom")
        except RuntimeError:
            pass
        assert "ex_cmd" not in SIGNATURES

    def test_nested_scopes_compose_and_unwind(self):
        outer = _stub("outer_cmd", ("script", "body", False))
        inner = _stub("inner_cmd", ("script", "body", False))
        with stub_signature_scope([outer]):
            assert "outer_cmd" in SIGNATURES
            assert "inner_cmd" not in SIGNATURES
            with stub_signature_scope([inner]):
                assert "outer_cmd" in SIGNATURES
                assert "inner_cmd" in SIGNATURES
            # Inner scope unwound; outer still active.
            assert "outer_cmd" in SIGNATURES
            assert "inner_cmd" not in SIGNATURES
        assert "outer_cmd" not in SIGNATURES

    def test_empty_stub_list_does_not_change_signatures(self):
        before = list(SIGNATURES.keys())
        with stub_signature_scope([]):
            assert list(SIGNATURES.keys()) == before
        assert list(SIGNATURES.keys()) == before

    def test_resolve_arg_role_map_sees_stub(self):
        stub = _stub("rolemap_cmd", ("sql", "value", False), ("script", "body", False))
        with stub_signature_scope([stub]):
            roles = resolve_arg_role_map("rolemap_cmd", ["x", "y"])
        assert roles == {1: frozenset({ArgRole.BODY})}

    def test_body_arg_indices_sees_stub(self):
        stub = _stub("body_cmd", ("a", "value", False), ("b", "body", False))
        with stub_signature_scope([stub]):
            assert body_arg_indices("body_cmd", ["x", "y"]) == {1}


class TestStubsInCallGraph:
    def test_switch_subject_calls_appear_as_edges(self):
        # Regression for the switch-subject path added alongside #409 —
        # ``switch [pick] { … }`` should record ``pick`` as a callee.
        source = textwrap.dedent("""\
            proc pick {} { return foo }
            proc handle_foo {} {}

            proc main {} {
                switch [pick] {
                    foo { handle_foo }
                }
            }
        """)
        data = build_call_graph(source)
        edge_pairs = {(e["caller"], e["callee"]) for e in data["edges"]}
        assert ("::main", "::pick") in edge_pairs
        assert ("::main", "::handle_foo") in edge_pairs

    def test_stub_with_two_body_args(self):
        # ``my_with {init:body finally:body}`` — both body args should be
        # walked so callees in either show up in the call graph.
        source = textwrap.dedent("""\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub my_with {init:body finally:body} -barrier
            # tcl-lsp: stubs-end

            proc setup {} {}
            proc teardown {} {}

            proc main {} {
                my_with {setup} {teardown}
            }
        """)
        data = build_call_graph(source)
        edge_pairs = {(e["caller"], e["callee"]) for e in data["edges"]}
        assert ("::main", "::setup") in edge_pairs
        assert ("::main", "::teardown") in edge_pairs

    def test_stub_callback_inside_catch_inside_if(self):
        # The fully-nested shape from issue #409: a stubbed command's
        # callback nested inside ``catch`` inside an ``if`` condition.
        source = textwrap.dedent("""\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub db_eval {sql script:body} -barrier
            # tcl-lsp: stubs-end

            proc on_row {} {}

            proc main {} {
                if {[catch {db_eval "SELECT 1" {on_row}}]} {}
            }
        """)
        data = build_call_graph(source)
        edge_pairs = {(e["caller"], e["callee"]) for e in data["edges"]}
        assert ("::main", "::on_row") in edge_pairs

    def test_stub_body_arg_resolves_namespaced_callbacks(self):
        # The callback is a namespaced proc — ensure name resolution
        # works inside the recursive scan.
        source = textwrap.dedent("""\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub db_eval {sql script:body} -barrier
            # tcl-lsp: stubs-end

            namespace eval ::rows {
                proc handle {} {}
            }

            proc main {} {
                db_eval "SELECT 1" {::rows::handle}
            }
        """)
        data = build_call_graph(source)
        edge_pairs = {(e["caller"], e["callee"]) for e in data["edges"]}
        assert ("::main", "::rows::handle") in edge_pairs

    def test_stub_does_not_leak_into_subsequent_analyses(self):
        # Analysis A declares a stub; analysis B does not.  B must not
        # see A's stub (otherwise the scope leaks across compilations).
        source_a = textwrap.dedent("""\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub leak_cmd {script:body}
            # tcl-lsp: stubs-end

            proc cb {} {}
            proc a {} { leak_cmd {cb} }
        """)
        source_b = textwrap.dedent("""\
            proc cb {} {}
            proc b {} { leak_cmd {cb} }
        """)
        build_call_graph(source_a)
        data_b = build_call_graph(source_b)
        edges_b = {(e["caller"], e["callee"]) for e in data_b["edges"]}
        # Without the stub, ``leak_cmd`` is an unknown command and the
        # ``{cb}`` argument is just a value — so no edge to cb.
        assert ("::b", "::cb") not in edges_b

    def test_callback_via_var_reference_is_not_synthesised(self):
        # ``db_eval $sql $script`` — the script arg is a variable
        # reference whose value isn't statically known, so the call
        # graph should NOT invent an edge (only literal bodies are
        # recursed into).
        source = textwrap.dedent("""\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub db_eval {sql script:body}
            # tcl-lsp: stubs-end

            proc cb {} {}

            proc main {sql script} {
                db_eval $sql $script
            }
        """)
        data = build_call_graph(source)
        edge_pairs = {(e["caller"], e["callee"]) for e in data["edges"]}
        assert ("::main", "::cb") not in edge_pairs


class TestSummaryCallsCoverage:
    def test_condition_call_is_recorded_on_caller_summary(self):
        # ``summary.calls`` feeds both the call-graph and the
        # unused-proc O124 optimisation.  This test asserts the field
        # directly so a regression of issue #409 (calls only inside an
        # ``if`` condition or switch subject) would surface even if
        # downstream consumers change.
        from core.compiler.compilation_unit import compile_source

        source = textwrap.dedent("""\
            proc q {} {}
            proc r {} {}
            proc s {} {}

            proc main {} {
                if {[q]} {}
                switch [r] { foo {} }
                s
            }
        """)
        cu = compile_source(source)
        calls = set(cu.interproc.procedures["::main"].calls)
        assert "::q" in calls
        assert "::r" in calls
        assert "::s" in calls

    def test_stubbed_callback_call_is_recorded_on_caller_summary(self):
        from core.compiler.compilation_unit import compile_source

        source = textwrap.dedent("""\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub db_eval {sql script:body} -barrier
            # tcl-lsp: stubs-end

            proc on_row {} {}

            proc main {} {
                db_eval "SELECT 1" {on_row}
            }
        """)
        cu = compile_source(source)
        assert "::on_row" in set(cu.interproc.procedures["::main"].calls)


class TestSubcommandStubs:
    """``stub <cmd> <subcmd> {args}`` produces a SubcommandSig overlay so
    one instance command can carry several subcommand signatures
    (e.g. ``db eval`` vs ``db transaction`` vs ``db function``).
    """

    def test_two_subcommands_dispatch_independently(self):
        eval_stub = _stub("db", ("sql", "value", False), ("script", "body", False))
        # Reconstruct with subcommand field set (the helper doesn't take it).
        eval_stub = StubCommandDef(
            name="db",
            subcommand="eval",
            args=eval_stub.args,
            range=_ZERO,
            barrier=True,
        )
        tx_stub = StubCommandDef(
            name="db",
            subcommand="transaction",
            args=(StubArgDef(name="script", role="body"),),
            range=_ZERO,
            barrier=True,
        )
        with stub_signature_scope([eval_stub, tx_stub]):
            # ``db eval $sql {body}`` — subcommand=eval, body at arg 1
            # within the eval subcommand (arg 2 overall, since arg 0 is
            # the subcommand word).
            eval_body = arg_indices_for_role("db", ["eval", "x", "{y}"], ArgRole.BODY)
            tx_body = arg_indices_for_role("db", ["transaction", "{y}"], ArgRole.BODY)
        assert eval_body == {2}
        assert tx_body == {1}

    def test_subcommand_stub_resolves_callgraph_edge(self):
        # Real ``db eval`` shape with a subcommand stub.  Body at args[2].
        source = textwrap.dedent("""\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub db eval {sql script:body} -barrier
            # tcl-lsp: stub db transaction {script:body} -barrier
            # tcl-lsp: stubs-end

            proc on_row {} {}
            proc do_tx {} {}

            proc dump {} {
                db eval "SELECT 1" {on_row}
            }
            proc run {} {
                db transaction {do_tx}
            }
        """)
        data = build_call_graph(source)
        edges = {(e["caller"], e["callee"]) for e in data["edges"]}
        assert ("::dump", "::on_row") in edges
        assert ("::run", "::do_tx") in edges


class TestOptionalArgRoleResolver:
    """Optional positional arguments (``?arg?``) shift the BODY index
    based on the actual call's argument count, so a single stub covers
    both ``cmd a body`` (2 args) and ``cmd a opt body`` (3 args).
    """

    def test_optional_middle_arg_shifts_body_position(self):
        stub = _stub(
            "cmd",
            ("sql", "value", False),
            ("rowvar", "value", True),
            ("script", "body", False),
        )
        with stub_signature_scope([stub]):
            two_args = arg_indices_for_role("cmd", ["x", "{y}"], ArgRole.BODY)
            three_args = arg_indices_for_role("cmd", ["x", "row", "{y}"], ArgRole.BODY)
        # Without rowvar: body at index 1.  With rowvar: body at index 2.
        assert two_args == {1}
        assert three_args == {2}

    def test_db_eval_rowvar_form_now_resolves(self):
        # The form previously documented as unsupported in
        # ``test_db_eval_rowvar_form_is_a_known_limitation``: with an
        # optional ``rowvar`` slot, ``db eval $sql rowvar {body}`` now
        # surfaces the callback in the call graph.  Subcommand stub
        # with optional-aware resolver.
        source = textwrap.dedent("""\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub db eval {sql ?rowvar? script:body} -barrier
            # tcl-lsp: stubs-end

            package require sqlite3
            sqlite3 db :memory:

            proc per_row {} {}

            proc dump_two_args {} {
                db eval {SELECT name FROM kv} {per_row}
            }
            proc dump_rowvar {} {
                db eval {SELECT name FROM kv} row {per_row}
            }
        """)
        data = build_call_graph(source)
        edges = {(e["caller"], e["callee"]) for e in data["edges"]}
        assert ("::dump_two_args", "::per_row") in edges
        assert ("::dump_rowvar", "::per_row") in edges


class TestRealisticSqliteUsage:
    """End-to-end coverage against the idioms ``sqlite3`` Tcl users write.

    The sqlite3 Tcl extension creates an instance command (``sqlite3 db
    :memory:`` → command ``db``) whose subcommands include ``eval``,
    ``onecolumn``, ``transaction``, ``function``, etc.  Common callback
    shapes from https://sqlite.org/tclsqlite.html :

      db eval $sql ?array? ?script?
      db transaction ?type? script
      db function NAME ?-argcount N? ?-deterministic? script

    The Tcl source in each test below was exercised against a real
    ``libsqlite3-tcl`` install (sqlite 3.45.1) and runs to completion —
    so we are verifying the analyser against the exact bytes the
    interpreter accepts.  Recognised idioms are positive assertions;
    the limitation cases assert the analyser does NOT invent edges
    when it cannot prove the body shape.
    """

    def test_db_eval_with_braced_callback_proc_name(self):
        # Verified runnable against real sqlite (tclsh + libsqlite3-tcl).
        # Body at args[2]; matches stub ``{subcommand sql script:body}``.
        source = textwrap.dedent("""\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub db {subcommand sql script:body} -barrier
            # tcl-lsp: stubs-end

            package require sqlite3
            sqlite3 db :memory:

            proc on_row {} { puts "$name = $value" }

            proc dump {} {
                db eval {SELECT name, value FROM kv} {on_row}
            }
        """)
        data = build_call_graph(source)
        edges = {(e["caller"], e["callee"]) for e in data["edges"]}
        assert ("::dump", "::on_row") in edges

    def test_db_eval_with_inline_multi_command_body(self):
        # The typical sqlite tutorial idiom — multiple commands inline
        # inside the row body.  Both calls should appear as edges.
        # Verified runnable against real sqlite.
        source = textwrap.dedent("""\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub db {subcommand sql script:body} -barrier
            # tcl-lsp: stubs-end

            package require sqlite3
            sqlite3 db :memory:

            proc render {} {}
            proc count {} {}

            proc dump {} {
                db eval {SELECT name, value FROM kv} {
                    render
                    count
                }
            }
        """)
        data = build_call_graph(source)
        edges = {(e["caller"], e["callee"]) for e in data["edges"]}
        assert ("::dump", "::render") in edges
        assert ("::dump", "::count") in edges

    def test_db_eval_inside_catch_inside_if(self):
        # ``if {[catch {db eval ...}]}`` — the full nested issue-#409
        # pattern applied to a realistic sqlite call site.
        source = textwrap.dedent("""\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub db {subcommand sql script:body} -barrier
            # tcl-lsp: stubs-end

            package require sqlite3
            sqlite3 db :memory:

            proc handle_row {} {}
            proc log_error {} {}

            proc safe_dump {} {
                if {[catch {db eval "SELECT * FROM t" {handle_row}}]} {
                    log_error
                }
            }
        """)
        data = build_call_graph(source)
        edges = {(e["caller"], e["callee"]) for e in data["edges"]}
        assert ("::safe_dump", "::handle_row") in edges
        assert ("::safe_dump", "::log_error") in edges

    def test_db_eval_rowvar_form_resolves_with_flat_stub_fallback(self):
        # With the flat positional stub ``{subcommand sql script:body}``,
        # the ``db eval $sql rowvar {body}`` form has body at args[3],
        # past the stub's static BODY index — so the call goes
        # unresolved (callback edge missing).  The recommended fix is
        # a subcommand stub with an optional ``?rowvar?`` slot — see
        # :class:`TestOptionalArgRoleResolver`.  This test pins the
        # behaviour of the flat-stub fallback so a regression of the
        # static-vs-dynamic dispatch is caught.
        source = textwrap.dedent("""\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub db {subcommand sql script:body} -barrier
            # tcl-lsp: stubs-end

            package require sqlite3
            sqlite3 db :memory:

            proc per_row {} {}

            proc dump_rowvar {} {
                db eval {SELECT name FROM kv} row {per_row}
            }
        """)
        data = build_call_graph(source)
        edges = {(e["caller"], e["callee"]) for e in data["edges"]}
        # Flat stub cannot model the variable BODY index; users hitting
        # this should switch to ``stub db eval {sql ?rowvar? script:body}``.
        assert ("::dump_rowvar", "::per_row") not in edges

    def test_user_wrapper_proc_is_the_recommended_workaround(self):
        # When the same instance command has multiple subcommands that
        # each take a body at different positions (``db eval``,
        # ``db transaction``, ``db function``…), a single flat stub
        # can't model them all.  The recommended pattern is to stub
        # both the instance command AND a thin wrapper proc.  Verified
        # runnable end-to-end against real sqlite (libsqlite3-tcl
        # 3.45.1) — the body of ``db_tx`` actually executes.
        source = textwrap.dedent("""\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub db {subcommand sql script:body} -barrier
            # tcl-lsp: stub db_tx {body:body} -barrier
            # tcl-lsp: stubs-end

            package require sqlite3
            sqlite3 db :memory:

            proc db_tx {body} {
                db transaction $body
            }

            proc tx_action_a {} {}
            proc tx_action_b {} {}

            proc run_changes {} {
                db_tx { tx_action_a; tx_action_b }
            }
        """)
        data = build_call_graph(source)
        edges = {(e["caller"], e["callee"]) for e in data["edges"]}
        assert ("::run_changes", "::tx_action_a") in edges
        assert ("::run_changes", "::tx_action_b") in edges

    def test_user_wrapper_around_sqlite_propagates_body_trait(self):
        # The wrapper's ``callback`` parameter flows into a stubbed
        # ``ArgRole.BODY`` slot, so ``infer_param_traits`` must mark
        # it as ``ProcArgTrait.BODY``.  This is what makes the wrapper
        # pattern compose — callers of ``each_row`` inherit BODY
        # recognition on their callback arg.
        from core.analysis.semantic_model import ProcArgTrait
        from core.compiler.compilation_unit import compile_source

        source = textwrap.dedent("""\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub db {subcommand sql script:body} -barrier
            # tcl-lsp: stubs-end

            package require sqlite3
            sqlite3 db :memory:

            proc each_row {sql callback} {
                db eval $sql $callback
            }
        """)
        cu = compile_source(source)
        each_row = cu.interproc.procedures["::each_row"]
        assert ProcArgTrait.BODY in each_row.param_traits.get("callback", frozenset())

    def test_unstubbed_sqlite_does_not_synthesise_edges(self):
        # Without the stub, the analyser has no way to know the third
        # word is a script, so it must NOT invent a call edge.  This
        # is what users see today on a fresh sqlite-using file before
        # they add a stub — they get unresolved-command diagnostics on
        # ``db`` and no spurious edges.
        source = textwrap.dedent("""\
            package require sqlite3
            sqlite3 db :memory:

            proc on_row {} {}

            proc dump {} {
                db eval {SELECT 1} {on_row}
            }
        """)
        data = build_call_graph(source)
        edges = {(e["caller"], e["callee"]) for e in data["edges"]}
        assert ("::dump", "::on_row") not in edges

    def test_dynamic_callback_variable_is_not_followed(self):
        # ``db eval $sql $cb`` — both args are variable references.
        # We document the limitation: the analyser only recurses into
        # literal braced/bareword bodies, not ``$var`` references whose
        # value isn't statically known.
        source = textwrap.dedent("""\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub db {subcommand sql script:body} -barrier
            # tcl-lsp: stubs-end

            package require sqlite3
            sqlite3 db :memory:

            proc on_row {} {}

            proc dump {cb} {
                set sql {SELECT 1}
                db eval $sql $cb
            }
        """)
        data = build_call_graph(source)
        edges = {(e["caller"], e["callee"]) for e in data["edges"]}
        # No edge to on_row — ``$cb`` doesn't resolve statically.  This
        # is the expected conservative behaviour, not a bug.
        assert ("::dump", "::on_row") not in edges


class TestStubTraitInference:
    def test_user_proc_passing_arg_to_stubbed_body_gets_body_trait(self):
        # If a user proc passes its parameter into a stubbed
        # ``ArgRole.BODY`` slot, the proc-arg trait inferencer should
        # mark that parameter as ``ProcArgTrait.BODY`` — exactly as it
        # does for built-in script-runners.
        from core.analysis.semantic_model import ProcArgTrait
        from core.compiler.compilation_unit import compile_source

        source = textwrap.dedent("""\
            # tcl-lsp: stubs-begin
            # tcl-lsp: stub db_eval {sql script:body}
            # tcl-lsp: stubs-end

            proc wrap {body} {
                db_eval "SELECT 1" $body
            }
        """)
        cu = compile_source(source)
        wrap = cu.interproc.procedures["::wrap"]
        assert ProcArgTrait.BODY in wrap.param_traits.get("body", frozenset())
