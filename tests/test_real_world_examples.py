"""Real-world regression tests for diagnostics and compiler optimisation passes."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.commands.registry.runtime import configure_signatures
from core.compiler.gvn import find_redundant_computations
from core.compiler.optimiser import find_optimisations, optimise_source
from lsp.features.diagnostics import get_diagnostics


def _example_text(*parts: str) -> str:
    root = Path(__file__).resolve().parent.parent
    return (root / "samples" / Path(*parts)).read_text()


def _diag_codes(source: str, *, optimiser_enabled: bool) -> set[str]:
    diags = get_diagnostics(source, optimiser_enabled=optimiser_enabled)
    return {str(d.code) for d in diags if d.code is not None}


@pytest.mark.parametrize(
    ("dialect", "path_parts", "expected_codes"),
    [
        (
            "f5-irules",
            ("irules", "diagnostics_event_flow.irul"),
            {
                "IRULE1001",
                "IRULE1002",
                "IRULE1003",
                "IRULE1004",
                "IRULE1005",
                "IRULE1006",
                "IRULE1201",
                "IRULE1202",
            },
        ),
        (
            "f5-irules",
            ("irules", "diagnostics_security.irul"),
            {
                "IRULE2003",
                "IRULE3001",
                "IRULE3002",
                "IRULE3003",
                "IRULE3101",
                "T100",
                "T102",
                "T200",
            },
        ),
        (
            "f5-irules",
            ("irules", "diagnostics_style_perf.irul"),
            {
                "IRULE2001",
                "IRULE2002",
                "IRULE2101",
                "IRULE4001",
                "IRULE4002",
                "IRULE5001",
                "T200",
            },
        ),
        (
            "tcl8.6",
            ("tcl", "05_warning_examples.tcl"),
            {"W100", "W201"},
        ),
        (
            "tcl8.6",
            ("tcl", "06_security_smells.tcl"),
            {"W101", "W102", "W103", "W300", "W301", "W303", "W304"},
        ),
    ],
)
def test_real_world_examples_emit_expected_preoptimisation_diagnostics(
    dialect: str,
    path_parts: tuple[str, ...],
    expected_codes: set[str],
) -> None:
    configure_signatures(dialect=dialect)
    source = _example_text(*path_parts)
    codes = _diag_codes(source, optimiser_enabled=False)
    assert expected_codes <= codes
    assert not any(code.startswith("O") for code in codes)


def test_preoptimisation_mode_excludes_o111_for_real_world_w100_example() -> None:
    configure_signatures(dialect="tcl8.6")
    source = _example_text("tcl", "05_warning_examples.tcl")

    preoptim_codes = _diag_codes(source, optimiser_enabled=False)
    full_codes = _diag_codes(source, optimiser_enabled=True)

    assert "W100" in preoptim_codes
    assert "O111" not in preoptim_codes
    assert "O111" in full_codes


def test_real_world_tcl_pipeline_covers_all_rewrite_passes() -> None:
    source = textwrap.dedent("""\
        proc add {a b} {
            return [expr {$a + $b}]
        }

        proc passthrough {x} {
            return $x
        }

        set timeout 30
        set half [expr {$timeout / 2}]
        set threshold [expr {$timeout + 10}]
        set candidate [expr {$request_count + 1 + 2}]
        set route [passthrough 42]

        set banner {Hello}
        append banner { }
        append banner World

        set stale 1
        set stale 2
        puts $stale

        set rolling 1
        set rolling [expr {$rolling + 1}]
        set rolling 5
        puts $rolling

        if {0} {
            puts never
            set never_seen 1
        }
        puts always

        if {[add 1 2] == 3} {
            puts matched
        }
    """)

    rewrites = find_optimisations(source)
    rewrite_codes = {r.code for r in rewrites}
    assert {
        "O101",
        "O102",
        "O103",
        "O104",
        "O108",
        "O109",
        "O110",
        "O112",
    } <= rewrite_codes

    optimised, _ = optimise_source(source)
    assert "set threshold 40" in optimised
    assert "set route 42" in optimised
    assert "set banner {Hello World}" in optimised
    assert "set rolling [expr {$rolling + 1}]" not in optimised


def test_real_world_irules_pipeline_covers_gvn_and_licm_passes() -> None:
    configure_signatures(dialect="f5-irules")
    source = textwrap.dedent("""\
        when HTTP_REQUEST priority 500 {
            set uri [HTTP::uri]
            set path_len [string length $uri]
            if {$path_len > 0} {
                set again [string length $uri]
            }
        }

        set host example.com
        for {set i 0} {$i < 3} {incr i} {
            set lower [string tolower $host]
            puts $lower
        }
    """)

    warnings = find_redundant_computations(source)
    codes = {w.code for w in warnings}
    assert {"O105", "O106"} <= codes
