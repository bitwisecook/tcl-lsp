"""f5report — interactive HTML reports for F5 BIG-IP configs.

The heavy lifting is done by the F5 BIG-IP *query engine* — the same
jq-flavoured ``f5-query`` DSL that ships with tcl-lsp — exposed to Python as a
native PyO3 extension (:mod:`f5report._engine`). Nothing here shells out to a
subprocess: configs and UCS archives are parsed, queried and projected entirely
in-process.

Typical use::

    import f5report
    sources = f5report.load_paths(["device.ucs"])
    html = f5report.build_report(sources, title="Production LTM")
    open("report.html", "w").write(html)
"""

from __future__ import annotations

from . import _engine
from ._engine import QueryError, load_paths, query, ucs_to_scf
from .report import build_report, collect_model

__all__ = [
    "QueryError",
    "load_paths",
    "query",
    "ucs_to_scf",
    "build_report",
    "collect_model",
    "engine_version",
]

__version__ = "0.1.0"


def engine_version() -> str:
    """Return the version of the native query-engine binding."""
    return _engine.__version__
