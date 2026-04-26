"""Differential parity harness for the C41 analyser Rust port.

The **parity tests** exercise both the Python implementation
(``core.analysis._analyser.Analyser.analyse``) and the Rust path
(via ``tcl_lsp_rust.analyser_analyse`` + the
``_materialise_rust_analysis`` helper) and assert structural
equivalence on the subset of :class:`AnalysisResult` fields the
Rust analyser populates.

The **dispatcher gating tests** assert the env-var-gated
dispatcher in ``core/analysis/_analyser/__init__.py::analyse``
behaves correctly. They patch the bound name ``_rust_analyse``
with a test double and never call the real binding, so they run
in Python-only environments too.

Tests that require the rust wheel call ``_require_rust_binding()``
on entry; tests that don't are unconditional. Mirrors the C40
``test_rust_signature_scan_differential.py`` pattern.
"""

from __future__ import annotations

import pytest


def _require_rust_binding():
    """Skip the calling test when the ``tcl_lsp_rust`` wheel is
    absent. Returns the module object on success."""
    return pytest.importorskip(
        "tcl_lsp_rust",
        reason="tcl_lsp_rust extension not built into this venv",
    )


from core.analysis._analyser import (  # noqa: E402
    Analyser,
    _materialise_rust_analysis,
)
from core.analysis.semantic_model import AnalysisResult  # noqa: E402


def _scope_shape(scope) -> dict:
    """Recursive shape descriptor for a :class:`Scope` — names
    only, no spans / ranges.

    The Rust analyser builds an isomorphic scope tree but the
    insertion order and exact span values can differ when the
    Python and Rust walkers visit children in slightly different
    orders.  Comparing shapes (kind / name / sorted child set /
    sorted variable+proc+class names) catches the structural
    parity gaps without false-positiving on cosmetic differences.
    """
    return {
        "kind": scope.kind,
        "name": scope.name,
        "variables": sorted(scope.variables.keys()),
        "procs": sorted(scope.procs.keys()),
        "classes": sorted(scope.classes.keys()),
        "children": sorted(
            (_scope_shape(c) for c in scope.children),
            key=lambda d: (d["kind"], d["name"]),
        ),
    }


def _diag_codes(result: AnalysisResult) -> list[str]:
    """Sorted list of diagnostic codes emitted by the analyser.

    Comparing the *set* of codes (not their messages or spans)
    catches Rust-vs-Python differences in which warnings fire
    while tolerating cosmetic message wording differences.
    """
    return sorted(d.code for d in result.diagnostics)


def _normalise_qname(qn: str) -> str:
    """Collapse repeated leading ``::`` to a single ``::``.

    Python's signature scanner emits ``::::ns::foo`` for
    ``proc ::ns::foo`` because it concatenates ``::`` with an
    already-qualified name.  The Rust port skips that double
    prefix.  Both shapes refer to the same proc — normalise
    before comparing.
    """
    while qn.startswith("::::"):
        qn = qn[2:]
    return qn


def _qname_set(d: dict) -> set[str]:
    return {_normalise_qname(k) for k in d}


def _compare_shapes(py: AnalysisResult, rust: AnalysisResult) -> None:
    """Assert *very coarse* structural parity on the most
    stable subset of :class:`AnalysisResult` fields the Rust
    port populates.

    The comparison is intentionally minimal — it checks that
    both paths recorded the *same set of top-level proc and
    class qualified names* (modulo the ``::::``/``::`` Python
    quirk).  Finer-grained equality (per-proc params, per-class
    methods, scope tree, ``all_variables`` membership,
    diagnostic-code parity) is the bar **after** the
    C41-default-on flip; the differential corpus's primary job
    today is to make sure the Rust path doesn't panic on a
    variety of inputs and produces structurally compatible
    output, not to lock the diagnostic-emitter set.
    """
    py_procs = _qname_set(py.all_procs)
    rust_procs = _qname_set(rust.all_procs)
    assert py_procs == rust_procs, (
        f"all_procs name-set mismatch: py={sorted(py_procs)} rust={sorted(rust_procs)}"
    )
    py_classes = _qname_set(py.all_classes)
    rust_classes = _qname_set(rust.all_classes)
    assert py_classes == rust_classes, (
        f"all_classes name-set mismatch: py={sorted(py_classes)} rust={sorted(rust_classes)}"
    )


def _fmt_range(r) -> tuple | None:
    """Compact tuple representation of a :class:`Range`, suitable
    for direct equality comparison."""
    if r is None:
        return None
    return (
        (r.start.line, r.start.character, r.start.offset),
        (r.end.line, r.end.character, r.end.offset),
    )


def _params_tuple(params) -> tuple:
    """Comparable tuple of :class:`ParamDef` fields."""
    return tuple((p.name, p.has_default, p.default_value) for p in params)


def _compare_fields(py: AnalysisResult, rust: AnalysisResult) -> None:
    """Strict per-field parity for the structural payload of
    :class:`AnalysisResult`.

    Compared:

    - ``all_procs`` keys (with the ``::::``/``::`` normalisation),
      and for each common qualified name the proc's bare ``name``,
      ``params`` tuple, ``name_range``, and ``body_range``.
    - ``all_classes`` keys, and for each common class the bare
      ``name``, ``name_range``, ``body_range``, ``methods`` /
      ``class_methods`` key sets, ``superclasses``, and ``mixins``.
    - The recursive scope-tree shape: ``kind``, ``name``, child
      variable / proc / class name sets at every level.

    Diagnostics are intentionally **not** compared field-by-field
    here. Python's ``Analyser().analyse`` integrates
    ``run_compiler_checks`` (which adds style + SSA-based dead-store
    diagnostics — W110, W220, W304, …); the Rust
    ``analyser_analyse`` is analyser-only. Mismatched diagnostic
    sets would fire on every fixture for an architectural reason
    rather than a regression. A future strip can layer a
    diagnostic-by-diagnostic comparison once the layering is
    aligned.
    """
    py_procs = {_normalise_qname(k): v for k, v in py.all_procs.items()}
    rust_procs = {_normalise_qname(k): v for k, v in rust.all_procs.items()}
    assert set(py_procs) == set(rust_procs), (
        f"all_procs keys: py={sorted(py_procs)} rust={sorted(rust_procs)}"
    )
    for qn in sorted(set(py_procs) & set(rust_procs)):
        p, r = py_procs[qn], rust_procs[qn]
        assert p.name == r.name, f"proc {qn} name: py={p.name!r} rust={r.name!r}"
        assert _params_tuple(p.params) == _params_tuple(r.params), (
            f"proc {qn} params: py={_params_tuple(p.params)} rust={_params_tuple(r.params)}"
        )
        assert _fmt_range(p.name_range) == _fmt_range(r.name_range), (
            f"proc {qn} name_range: py={_fmt_range(p.name_range)} rust={_fmt_range(r.name_range)}"
        )
        assert _fmt_range(p.body_range) == _fmt_range(r.body_range), (
            f"proc {qn} body_range: py={_fmt_range(p.body_range)} rust={_fmt_range(r.body_range)}"
        )

    py_classes = {_normalise_qname(k): v for k, v in py.all_classes.items()}
    rust_classes = {_normalise_qname(k): v for k, v in rust.all_classes.items()}
    assert set(py_classes) == set(rust_classes), (
        f"all_classes keys: py={sorted(py_classes)} rust={sorted(rust_classes)}"
    )
    for qn in sorted(set(py_classes) & set(rust_classes)):
        p, r = py_classes[qn], rust_classes[qn]
        assert p.name == r.name, f"class {qn} name: py={p.name!r} rust={r.name!r}"
        assert _fmt_range(p.name_range) == _fmt_range(r.name_range), (
            f"class {qn} name_range: py={_fmt_range(p.name_range)} rust={_fmt_range(r.name_range)}"
        )
        assert _fmt_range(p.body_range) == _fmt_range(r.body_range), (
            f"class {qn} body_range: py={_fmt_range(p.body_range)} rust={_fmt_range(r.body_range)}"
        )
        assert sorted(p.methods) == sorted(r.methods), (
            f"class {qn} methods: py={sorted(p.methods)} rust={sorted(r.methods)}"
        )
        assert sorted(p.class_methods) == sorted(r.class_methods), (
            f"class {qn} class_methods: py={sorted(p.class_methods)} rust={sorted(r.class_methods)}"
        )
        assert list(p.superclasses) == list(r.superclasses), (
            f"class {qn} superclasses: py={p.superclasses} rust={r.superclasses}"
        )
        assert list(p.mixins) == list(r.mixins), f"class {qn} mixins: py={p.mixins} rust={r.mixins}"

    _compare_scope_tree(py.global_scope, rust.global_scope, path="<global>")


def _compare_scope_tree(py_scope, rust_scope, *, path: str) -> None:
    """Recursive scope-shape parity: kind, name, var / proc /
    class names at every level. Child scopes are matched by
    ``(kind, name)`` so insertion order doesn't matter."""
    assert py_scope.kind == rust_scope.kind, (
        f"scope {path} kind: py={py_scope.kind!r} rust={rust_scope.kind!r}"
    )
    assert py_scope.name == rust_scope.name, (
        f"scope {path} name: py={py_scope.name!r} rust={rust_scope.name!r}"
    )
    assert sorted(py_scope.variables) == sorted(rust_scope.variables), (
        f"scope {path} variables: py={sorted(py_scope.variables)} "
        f"rust={sorted(rust_scope.variables)}"
    )
    assert sorted(py_scope.procs) == sorted(rust_scope.procs), (
        f"scope {path} procs: py={sorted(py_scope.procs)} rust={sorted(rust_scope.procs)}"
    )
    assert sorted(py_scope.classes) == sorted(rust_scope.classes), (
        f"scope {path} classes: py={sorted(py_scope.classes)} rust={sorted(rust_scope.classes)}"
    )
    py_children = {(c.kind, c.name): c for c in py_scope.children}
    rust_children = {(c.kind, c.name): c for c in rust_scope.children}
    assert set(py_children) == set(rust_children), (
        f"scope {path} child set: py={sorted(py_children)} rust={sorted(rust_children)}"
    )
    for key in sorted(py_children.keys() & rust_children.keys()):
        _compare_scope_tree(
            py_children[key],
            rust_children[key],
            path=f"{path}/{key[0]}:{key[1]}",
        )


def _assert_same(source: str) -> None:
    """Run both implementations on ``source`` and assert shape parity.

    Skips when the rust binding isn't built.
    """
    tcl_lsp_rust = _require_rust_binding()
    py = Analyser().analyse(source)
    rust_raw = tcl_lsp_rust.analyser_analyse(source, "tcl")
    rust = _materialise_rust_analysis(source, rust_raw)
    _compare_shapes(py, rust)


# Inline corpus — one tuple per fixture. Each tuple is
# ``(label, source)``; the label drives the pytest test ID so
# failures point at the offending shape directly.
#
# Fixtures span every W-code family the Rust port emits + every
# OO subcommand + the recovery shapes (C41e4 / C41e5).  Mined
# from the broader ``tests/test_analyser.py`` corpus; the
# C41-default-on flip will gate on this set converging.
CORPUS: list[tuple[str, str]] = [
    # -- procs --
    ("bare_proc", "proc foo {} {}"),
    ("proc_with_params", "proc add {a b} { return [expr {$a + $b}] }"),
    ("proc_with_default", "proc greet {name {greeting Hello}} { puts $greeting }"),
    ("proc_qualified_name", "proc ::ns::foo {} {}"),
    ("proc_inside_namespace_eval", "namespace eval ns { proc inner {} {} }"),
    # -- variables --
    ("set_global", "set x 1"),
    ("set_qualified", "set ::ns::x 1"),
    ("incr_init", "incr counter"),
    ("set_then_read", "set x 1\nputs $x"),
    ("multiple_globals", "set x 1\nset y 2\nset z 3"),
    # -- W113 (proc shadows builtin) --
    ("w113_proc_shadows_set", "proc set {} {}"),
    ("w113_proc_shadows_puts", "proc puts {} {}"),
    # -- W123 (unresolved command) --
    ("w123_unknown_command", "totally_not_a_command arg1 arg2"),
    ("w123_user_proc_resolves", "proc helper {} {}\nhelper"),
    # -- W210 (read-before-set) --
    ("w210_read_before_set_proc", "proc foo {} { puts $undef }"),
    ("w210_read_before_set_top_level", "puts $undef_top"),
    (
        "w210_suppressed_when_proc_writes_global",
        "proc init {} { set ::counter 0 }\nputs $counter",
    ),
    # -- W211 (unused variable) --
    ("w211_unused_var", "proc foo {} { set x 1 }"),
    # -- W213 (unset on possibly undefined) --
    ("w213_unset_no_complain", "proc foo {} { unset xs }"),
    # -- OO classes --
    ("oo_class_empty", "oo::class create C {}"),
    ("oo_class_with_method", "oo::class create C { method greet {} { puts hi } }"),
    (
        "oo_class_with_superclass",
        "oo::class create Sub { superclass ::Base }",
    ),
    (
        "oo_class_with_mixin",
        "oo::class create C { mixin ::M }",
    ),
    (
        "oo_class_with_constructor",
        "oo::class create C { constructor args { puts ctor } }",
    ),
    (
        "oo_class_with_destructor",
        "oo::class create C { destructor { puts dtor } }",
    ),
    (
        "oo_class_with_property",
        "oo::class create C { property colour -kind readable }",
    ),
    (
        "oo_class_with_filter",
        "oo::class create C { filter log }",
    ),
    (
        "oo_class_with_export",
        "oo::class create C { export foo bar }",
    ),
    (
        "oo_define_extends_existing",
        "oo::class create C {}\noo::define C { method m {} {} }",
    ),
    (
        "oo_define_inline_form",
        "oo::define MyClass method greet {} { puts hi }",
    ),
    # -- unknown proc detection --
    (
        "unknown_proc_with_switch",
        "proc unknown {cmd args} { switch -exact $cmd { foo { return 1 } bar { return 2 } } }",
    ),
    ("unknown_proc_empty_stub", "proc unknown {cmd args} {}"),
    (
        "unknown_proc_with_exec",
        "proc unknown {cmd args} { exec $cmd }",
    ),
    # -- package require --
    ("package_require_with_version", "package require Tcl 8.6"),
    ("package_require_exact", "package require -exact Tcl 8.6"),
    # -- namespaces --
    (
        "namespace_eval_with_proc",
        "namespace eval pkg { proc helper {} {} }",
    ),
    # The two fixtures below pin the Python parity bugs fixed
    # alongside the C41-default-on flip: ``proc ::ns::foo`` no
    # longer yields a stray ``::::ns::foo`` qualified-name key, and
    # ``namespace eval outer { namespace eval inner { ... } }`` no
    # longer drops the outer namespace.
    (
        "nested_namespace_eval_proc",
        "namespace eval outer { namespace eval inner { proc deep {} {} } }",
    ),
    (
        "nested_namespace_eval_class",
        "namespace eval outer { namespace eval inner { oo::class create C {} } }",
    ),
    # -- control flow --
    (
        "for_loop_with_inner_set",
        "for {set i 0} {$i < 10} {incr i} { puts $i }",
    ),
    (
        "foreach_loop",
        "foreach x {1 2 3} { puts $x }",
    ),
    (
        "switch_form_2",
        "switch $x { a { puts a } b { puts b } default { puts other } }",
    ),
    # -- W124 (IP literal compare in expr) --
    (
        "w124_ip_literal_compare",
        'proc f {} { set ip "10.0.0.1"; if {$ip eq "192.168.1.1"} { puts yes } }',
    ),
    # -- W214 (unused proc parameter) --
    ("w214_unused_first_param", "proc f {a b} { puts $b }"),
    ("w214_unused_only_param", "proc f {x} {}"),
    # -- H300 (case-mismatch hint) --
    (
        "h300_case_mismatch_both_used",
        "proc f {} { set Foo 1; puts $foo; puts $Foo }",
    ),
    (
        "h300_case_mismatch_proc_param",
        "proc render {Total} { return $total }",
    ),
    # -- E101 (recovery: unclosed proc body) --
    ("e101_unclosed_proc_body", "proc foo {} {\nset x 1\n"),
    ("e101_unclosed_proc_body_short", "proc foo {} { puts hi\n"),
    # -- combined --
    (
        "package_plus_proc_plus_class",
        """\
package require Tcl 8.6
namespace eval my::ns {
    proc helper {} { return 1 }
    oo::class create Widget {
        method draw {} { puts drawing }
    }
}
""",
    ),
]


@pytest.mark.parametrize(
    ("label", "source"),
    CORPUS,
    ids=[label for label, _ in CORPUS],
)
def test_analyser_shape_matches_python(label: str, source: str) -> None:
    """Structural-shape parity for every corpus fixture.

    The Rust port populates a subset of ``AnalysisResult`` —
    procs, classes, variables, scope tree, and a subset of
    diagnostics.  We assert the populated subset matches
    Python.  Field-by-field equality is the bar after C41-default-on
    has baked; for now ``_compare_shapes`` is shape-only.
    """
    del label  # only used as the pytest id
    _assert_same(source)


# Field-by-field corpus — strict per-fixture parity on the
# structural payload of :class:`AnalysisResult` (proc / class
# fields, scope tree). The set is intentionally a subset of
# ``CORPUS``: each fixture here is one we've audited as passing
# the strict comparator. New fixtures graduate from
# ``CORPUS`` (shape-only) to this list as parity gaps close.
#
# Diagnostic-by-diagnostic parity is **not** included — Python's
# ``Analyser().analyse`` integrates ``run_compiler_checks``
# (style + SSA dead-store diagnostics) while the Rust
# ``analyser_analyse`` is analyser-only; layering alignment is a
# follow-up chunk.
FIELD_PARITY_LABELS: frozenset[str] = frozenset(
    {
        "bare_proc",
        "proc_with_params",
        "proc_with_default",
        "proc_qualified_name",
        "proc_inside_namespace_eval",
        "set_global",
        "set_qualified",
        "incr_init",
        "set_then_read",
        "multiple_globals",
        "w113_proc_shadows_set",
        "w113_proc_shadows_puts",
        "w123_unknown_command",
        "w123_user_proc_resolves",
        "w210_read_before_set_proc",
        "w210_read_before_set_top_level",
        "w210_suppressed_when_proc_writes_global",
        "w211_unused_var",
        "unknown_proc_with_switch",
        "unknown_proc_empty_stub",
        "unknown_proc_with_exec",
        "package_require_with_version",
        "package_require_exact",
        "namespace_eval_with_proc",
        "nested_namespace_eval_proc",
        "foreach_loop",
        "switch_form_2",
        "w124_ip_literal_compare",
        "w214_unused_first_param",
        "w214_unused_only_param",
        "h300_case_mismatch_both_used",
        "h300_case_mismatch_proc_param",
        "e101_unclosed_proc_body",
        "e101_unclosed_proc_body_short",
    }
)

# Fixtures excluded from :data:`FIELD_PARITY_LABELS` and the gap
# they currently exercise:
#
# - **OO fixtures** (``oo_class_*``, ``oo_define_*``,
#   ``nested_namespace_eval_class``, ``package_plus_proc_plus_class``).
#   The Rust analyser populates ``result.all_classes`` (so the
#   qualified-name + scope-tree top-level comparison succeeds) but
#   does not yet thread class definitions back into the per-scope
#   ``Scope.classes`` map.
# - ``w213_unset_no_complain`` — the Rust port doesn't record an
#   ``unset xs`` argument as a per-scope variable, so the proc
#   scope's variable set diverges.
# - ``for_loop_with_inner_set`` — the Rust port doesn't surface
#   the ``set i 0`` from the for-loop init body in the enclosing
#   scope's variable set.
#
# Each gap is a Rust-side follow-up; fixtures graduate to
# :data:`FIELD_PARITY_LABELS` as the gaps close.


@pytest.mark.parametrize(
    ("label", "source"),
    [(label, source) for label, source in CORPUS if label in FIELD_PARITY_LABELS],
    ids=[label for label, _ in CORPUS if label in FIELD_PARITY_LABELS],
)
def test_analyser_field_matches_python(label: str, source: str) -> None:
    """Strict field-by-field parity for the audited subset
    (:data:`FIELD_PARITY_LABELS`) of fixtures."""
    del label  # only used as the pytest id
    tcl_lsp_rust = _require_rust_binding()
    py = Analyser().analyse(source)
    rust_raw = tcl_lsp_rust.analyser_analyse(source, "tcl")
    rust = _materialise_rust_analysis(source, rust_raw)
    _compare_fields(py, rust)


# Dispatcher gating tests. The corpus tests above call the Rust
# path directly; these assert the env-var-gated dispatcher in
# ``core/analysis/_analyser/__init__.py::analyse`` behaves
# correctly.

import core.analysis._analyser as analyser_module  # noqa: E402
from core.analysis._analyser import analyse  # noqa: E402


def _sentinel_raw() -> dict:
    """A skeletal ``analyser_analyse`` return value with a
    well-known proc name.  Patched in via monkeypatch so the
    dispatcher gating tests don't need the real Rust binding."""
    return {
        "global_scope": {
            "kind": "global",
            "name": "::",
            "body_range": None,
            "variables": {},
            "procs": {
                "sentinel": {
                    "name": "sentinel",
                    "qualified_name": "::sentinel",
                    "params": [],
                    "name_range": (0, 8),
                    "body_range": (10, 20),
                    "doc": "",
                },
            },
            "classes": {},
            "children": [],
        },
        "all_procs": {
            "::sentinel": {
                "name": "sentinel",
                "qualified_name": "::sentinel",
                "params": [],
                "name_range": (0, 8),
                "body_range": (10, 20),
                "doc": "",
            },
        },
        "all_classes": {},
        "all_variables": {},
        "diagnostics": [],
        "command_invocations": [],
        "package_requires": [],
        "source_targets": [],
        "command_aliases": {},
        "namespace_imports": [],
        "unknown_proc_info": None,
    }


def test_dispatcher_default_off_uses_python(monkeypatch) -> None:
    """With the env var unset, ``analyse`` must dispatch to the
    Python path (default-OFF polarity at C41f6).  Asserted by
    patching ``_rust_analyse`` with a sentinel and confirming the
    real Python proc name (``::foo``) — not the sentinel — is
    populated."""
    monkeypatch.delenv("TCL_LSP_RUST_ANALYSER", raising=False)
    monkeypatch.setattr(analyser_module, "_rust_analyse", lambda _src, _dia: _sentinel_raw())
    result = analyse("proc foo {} {}")
    # Python path produces ::foo; sentinel absent proves Python ran.
    assert "::foo" in result.all_procs
    assert "::sentinel" not in result.all_procs


def test_dispatcher_env_var_on_uses_rust(monkeypatch) -> None:
    """``TCL_LSP_RUST_ANALYSER=1`` (explicit opt-in) flips to the
    Rust dispatch — sentinel proves the binding ran instead of
    the Python implementation."""
    monkeypatch.setenv("TCL_LSP_RUST_ANALYSER", "1")
    monkeypatch.setattr(analyser_module, "_rust_analyse", lambda _src, _dia: _sentinel_raw())
    result = analyse("source ignored — patched rust returns sentinel")
    assert "::sentinel" in result.all_procs
    assert "::foo" not in result.all_procs


def test_dispatcher_env_var_zero_keeps_python(monkeypatch) -> None:
    """``TCL_LSP_RUST_ANALYSER=0`` is explicit opt-out — must
    keep the Python path even when patched ``_rust_analyse`` is
    available."""
    monkeypatch.setenv("TCL_LSP_RUST_ANALYSER", "0")
    monkeypatch.setattr(analyser_module, "_rust_analyse", lambda _src, _dia: _sentinel_raw())
    result = analyse("proc foo {} {}")
    assert "::foo" in result.all_procs
    assert "::sentinel" not in result.all_procs


def test_dispatcher_falls_back_when_rust_raises(monkeypatch) -> None:
    """When the Rust path raises, the dispatcher must catch the
    exception, log at DEBUG, and fall back to the Python
    implementation.  Otherwise a Rust-side crash would silently
    break analysis."""
    monkeypatch.setenv("TCL_LSP_RUST_ANALYSER", "1")

    def _boom(_src, _dia):
        raise RuntimeError("rust path exploded")

    monkeypatch.setattr(analyser_module, "_rust_analyse", _boom)
    result = analyse("proc foo {} {}")
    # Python fallback ran ⇒ ::foo present.
    assert "::foo" in result.all_procs
