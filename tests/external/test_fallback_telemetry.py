"""Regression tests for Tcl 9 fallback-site telemetry (triage).

These tests cover the pre-run ``DiagMap`` summarisation fed into
``tmp/tcl9-report.json``: a compile of a script that cannot be
specialised produces a non-zero fallback count, and the count is
bucketed by command + kind so the triage renderer can surface
per-file coverage.
"""

from __future__ import annotations

from scripts.tcl9_triage_report import _render_table
from tests.external.run_tcl9_tests import _summarise_diag
from tests.test_wasm_real_tcl import _compile_tcl_with_diag


def test_unknown_command_registers_fallback_site() -> None:
    """An unknown command lowers to IRBarrier and emits a diag site."""
    _wasm, diag = _compile_tcl_with_diag("foo 1 2\n", "unknown_cmd.tcl")
    summary = _summarise_diag(diag)
    assert summary["fallback_sites_total"] >= 1
    # Every site is counted; "foo" is the only command here that
    # could trigger a fallback.
    commands = [cmd for cmd, _ in summary["top_fallback_commands"]]
    assert "foo" in commands


def test_summary_empty_for_pure_compile() -> None:
    """``set x 1`` lowers entirely inline; no diag sites emitted."""
    _wasm, diag = _compile_tcl_with_diag("set x 1\n", "pure.tcl")
    summary = _summarise_diag(diag)
    assert summary["fallback_sites_total"] == 0
    assert summary["fallback_sites_by_kind"] == {}
    assert summary["top_fallback_commands"] == []


def test_nontrapping_runtime_command_skips_diag_site() -> None:
    """``puts`` / ``append`` are classified non-trapping — no diag site.

    Pure ``puts hello`` compiles through the runtime dispatch path
    (``_emit_cmd_runtime``), which now skips the per-call
    ``tcl_diag_set`` preamble for commands in
    ``_CMD_RUNTIME_NONTRAPPING``.  The emitted DiagMap must have zero
    sites for the compile.
    """
    _wasm, diag = _compile_tcl_with_diag("puts hello\n", "puts.tcl")
    summary = _summarise_diag(diag)
    assert summary["fallback_sites_total"] == 0

    # ``append x "one"`` is statement-context mutate-var path — also
    # goes through runtime dispatch without a diag site now.
    _wasm, diag = _compile_tcl_with_diag('append x "one"\n', "append.tcl")
    summary = _summarise_diag(diag)
    assert summary["fallback_sites_total"] == 0


def test_trapping_runtime_command_keeps_diag_site() -> None:
    """Stub commands that raise ``unsupported command: X`` still
    record a diag site so the stderr trap line resolves to source.

    ``fconfigure`` routes through ``_CMD_RUNTIME`` but is NOT in
    ``_CMD_RUNTIME_NONTRAPPING``: the compile emits a single
    ``runtime``-kind diag site for it.
    """
    _wasm, diag = _compile_tcl_with_diag("fconfigure stdin -blocking 0\n", "trap_fconfigure.tcl")
    runtime_sites = [s for s in diag.sites if s.kind == "runtime"]
    assert any(s.command == "fconfigure" for s in runtime_sites), (
        "fconfigure must keep its diag site (stub raises unsupported "
        "command and stderr trap line needs to resolve)"
    )


def test_summary_none_diag_returns_zeroes() -> None:
    """Compile failed before a DiagMap could be produced — all zeroes."""
    summary = _summarise_diag(None)
    assert summary == {
        "fallback_sites_total": 0,
        "fallback_sites_by_kind": {},
        "top_fallback_commands": [],
    }


def test_triage_table_renders_fb_column() -> None:
    """The rendered triage markdown must carry the FB column and value."""
    entries = [
        {
            "file": "foo.test",
            "subsystem": "string",
            "stage": "run",
            "status": "fail",
            "category": "A",
            "total": 10,
            "passed": 7,
            "failed": 3,
            "first_failing_test": "foo-1.1",
            "trap_site": None,
            "fallback_sites_total": 42,
        }
    ]
    rendered = _render_table(entries)
    assert "| FB |" in rendered, "FB column header missing from rendered table"
    assert "| 42 |" in rendered, "FB column value missing from rendered row"
    assert "fallback_sites=42" in rendered, "totals line missing fallback_sites"


def test_triage_table_totals_sum_fallback_sites() -> None:
    """The totals line must sum fallback_sites across all rows."""
    entries = [
        {
            "file": "a.test",
            "subsystem": "list",
            "stage": "run",
            "status": "fail",
            "category": "A",
            "fallback_sites_total": 3,
        },
        {
            "file": "b.test",
            "subsystem": "list",
            "stage": "run",
            "status": "fail",
            "category": "B",
            "fallback_sites_total": 7,
        },
        {
            "file": "c.test",
            "subsystem": "list",
            "stage": "run",
            "status": "pass",
            "category": "pass",
            # Missing field — must be treated as zero.
        },
    ]
    rendered = _render_table(entries)
    assert "fallback_sites=10" in rendered


def _non_namespace_fallback_sites(diag) -> int:
    """Count fallback sites whose command is NOT a ``namespace ...``
    directive.  ``namespace import`` / ``export`` / ``forget`` now
    route through an eval fallback on purpose — the runtime needs
    to see them so its ``ns_import`` / ``ns_export`` create real
    redirects for the eval-fallback dispatch path to resolve
    imported names after a full flush (``interp create`` etc.).
    Those sites are expected and shouldn't count against the
    "resolve unqualified calls directly" invariant these tests
    pin.
    """
    if diag is None or not diag.sites:
        return 0
    return sum(1 for site in diag.sites if site.kind == "fallback" and site.command != "namespace")


def test_namespace_import_resolves_unqualified_calls() -> None:
    """``namespace import ::foo::*`` eliminates the fallback for bare calls.

    Compiling ``namespace eval ::foo { proc bar {} {...} }; namespace
    import ::foo::*; bar`` used to emit a ``tcl_eval`` fallback for
    the bare ``bar`` call because the compiler couldn't see that
    ``bar`` was imported from ``::foo``.  With the import table
    wired through, the call resolves to ``::foo::bar`` at compile
    time and dispatches directly — zero diag sites for the
    imported-call path.  (``namespace import`` itself now emits a
    runtime fallback so the Zig runtime creates the redirect;
    those sites are excluded from this check.)
    """
    src = (
        "namespace eval ::foo {\n"
        "    proc bar {} { return 1 }\n"
        "    proc baz {} { return 2 }\n"
        "    namespace export bar baz\n"
        "}\n"
        "namespace import ::foo::*\n"
        "bar\n"
        "baz\n"
    )
    _wasm, diag = _compile_tcl_with_diag(src, "ns_import.tcl")
    non_ns = _non_namespace_fallback_sites(diag)
    assert non_ns == 0, (
        f"expected zero non-namespace fallback sites after namespace "
        f"import, got {non_ns}: {_summarise_diag(diag)['top_fallback_commands']}"
    )


def test_namespace_import_single_name_resolves() -> None:
    """Single-name import (``namespace import ::foo::bar``) also works.

    The source namespace must ``namespace export bar`` for the
    import shortcut to fire — otherwise codegen correctly falls
    back to runtime dispatch (which matches C Tcl's
    ``Tcl_Import`` semantics: unexported names can't be imported).
    """
    src = (
        "namespace eval ::foo {\n"
        "    proc bar {} { return 1 }\n"
        "    proc baz {} { return 2 }\n"
        "    namespace export bar\n"
        "}\n"
        "namespace import ::foo::bar\n"
        "bar\n"
    )
    _wasm, diag = _compile_tcl_with_diag(src, "ns_import_single.tcl")
    non_ns = _non_namespace_fallback_sites(diag)
    assert non_ns == 0


def test_namespace_import_without_export_falls_back() -> None:
    """Importing an unexported name must keep the runtime dispatch
    path so the interpreter can apply the correct "unknown
    command" diagnostic.  This is the P8-review correctness fix:
    the compile-time shortcut is now gated on ``namespace export``
    patterns, matching C Tcl's ``Tcl_Import`` rules.
    """
    src = (
        "namespace eval ::foo {\n"
        "    proc bar {} { return 1 }\n"
        # No ``namespace export bar`` — bar is NOT importable.
        "}\n"
        "namespace import ::foo::bar\n"
        "bar\n"
    )
    _wasm, diag = _compile_tcl_with_diag(src, "ns_import_unexported.tcl")
    summary = _summarise_diag(diag)
    # The ``bar`` call falls back to runtime dispatch; at least
    # one fallback site must remain.
    assert summary["fallback_sites_total"] >= 1


def test_unimported_name_still_falls_back() -> None:
    """``baz`` was not imported, so bare ``baz`` must still fall back.

    Guards against the import table being too permissive — only
    names present in a recorded ``namespace import`` directive
    should dispatch directly.
    """
    src = (
        "namespace eval ::foo {\n"
        "    proc bar {} { return 1 }\n"
        "    proc baz {} { return 2 }\n"
        "}\n"
        "namespace import ::foo::bar\n"
        "baz\n"  # not imported — must fall back
    )
    _wasm, diag = _compile_tcl_with_diag(src, "ns_import_partial.tcl")
    summary = _summarise_diag(diag)
    assert summary["fallback_sites_total"] >= 1
    commands = [c for c, _ in summary["top_fallback_commands"]]
    assert "baz" in commands


def test_dead_code_namespace_import_falls_back() -> None:
    """Regression for `#189 <https://github.com/bitwisecook/tcl-lsp/issues/189>`_.

    A ``namespace import`` buried inside ``if {0} { ... }`` must
    NOT arm the compile-time dispatch shortcut, because the
    corresponding ``namespace eval`` in the same dead block never
    runs at runtime — the imported command isn't defined.
    Compiling the bare call ``evil`` must leave a runtime fallback
    site so the interpreter surfaces "unknown command: evil".
    """
    src = (
        "if {0} {\n"
        "    namespace eval ::other {\n"
        "        namespace export evil\n"
        "        proc evil {} { return bad }\n"
        "    }\n"
        "    namespace import ::other::evil\n"
        "}\n"
        "evil\n"
    )
    _wasm, diag = _compile_tcl_with_diag(src, "ns_import_dead.tcl")
    summary = _summarise_diag(diag)
    # ``evil`` must retain a runtime-fallback site.
    commands = [c for c, _ in summary["top_fallback_commands"]]
    assert "evil" in commands, (
        "dead-code namespace import incorrectly globalised ``evil``: "
        f"{summary['top_fallback_commands']}"
    )


def test_kind_bucketing_distinguishes_fallback_from_unsupported() -> None:
    """Kind histogram must separate fallback from unsupported sites.

    Any script that compiles but cannot specialise and must defer to
    the runtime should register at least one ``fallback`` site. The
    histogram is keyed by ``DiagSite.kind`` so the triage table can
    track architectural dead-ends separately from relaxable barriers.
    """
    _wasm, diag = _compile_tcl_with_diag(
        "proc p {} { unknown_user_cmd a b }\np\n",
        "kinds.tcl",
    )
    summary = _summarise_diag(diag)
    assert summary["fallback_sites_total"] >= 1
    assert "fallback" in summary["fallback_sites_by_kind"]
    assert summary["fallback_sites_by_kind"]["fallback"] >= 1
