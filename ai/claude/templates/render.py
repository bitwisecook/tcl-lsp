"""Jinja2-based iRule test script renderer.

Loads templates from the templates/ package directory and provides
``render_test_script`` as the public API.  An optional post-render
formatting pass uses ``core.formatting.formatter.format_tcl``.
"""

from __future__ import annotations

from pathlib import Path

import jinja2

_TEMPLATE_DIR = Path(__file__).resolve().parent
_env: jinja2.Environment | None = None


def _get_env() -> jinja2.Environment:
    global _env
    if _env is None:
        _env = jinja2.Environment(
            loader=jinja2.FileSystemLoader(str(_TEMPLATE_DIR)),
            keep_trailing_newline=True,
            lstrip_blocks=True,
            trim_blocks=True,
        )
    return _env


def render_test_case(
    *,
    test_id: str,
    desc: str,
    body: list[str],
) -> str:
    """Render a single ``::orch::test`` block."""
    tmpl = _get_env().get_template("test_case.tcl.j2")
    return tmpl.render(test_id=test_id, desc=desc, body=body)


def render_test_script(
    *,
    test_name: str,
    basename: str,
    source: str,
    profiles: list[str],
    setup_lines: list[str],
    test_blocks: list[str],
    multi_tmm: dict | None = None,
    format_output: bool = True,
) -> str:
    """Render a complete iRule test script from structured data.

    Parameters
    ----------
    test_name:
        Base name for test IDs (``basename`` minus extension).
    basename:
        Original iRule file name.
    source:
        Raw iRule source code (embedded in the test).
    profiles:
        List of profile strings (e.g. ``["TCP", "HTTP"]``).
    setup_lines:
        Pre-indented setup body lines (pools, data groups, statics).
    test_blocks:
        Pre-rendered ``::orch::test`` blocks (from ``render_test_case``
        or ``_gen_cfg_test_cases``).
    multi_tmm:
        If not ``None``, a dict with keys ``source``, ``profiles_str``,
        ``pools``, ``test_name``, ``check_var`` for the multi-TMM section.
    format_output:
        Run the result through ``format_tcl`` for consistent style.

    Returns
    -------
    str
        The complete rendered (and optionally formatted) test script.
    """
    tmpl = _get_env().get_template("irule_test.tcl.j2")
    result = tmpl.render(
        test_name=test_name,
        basename=basename,
        source=source.strip(),
        profiles=profiles,
        setup_lines=setup_lines,
        test_blocks=test_blocks,
        multi_tmm=multi_tmm,
    )

    if format_output:
        result = _format_filter(result)

    return result


def _format_filter(source: str) -> str:
    """Apply the Tcl formatter as a post-generation filter.

    Silently returns the original source if the formatter is unavailable
    or raises an error (e.g. on malformed generated output).
    """
    try:
        from core.formatting import format_tcl

        return format_tcl(source)
    except Exception:
        return source
