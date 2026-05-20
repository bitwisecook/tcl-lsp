"""The byte_compiled / not_proc_factory / frameless_runtime traits are the
single source of truth, replacing the former consumer-local name lists.

Each test pins the registry-derived set to the exact audited list the
consumer used to hardcode, so a stray stamp (or a missing one) is caught.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.registry import REGISTRY

# Authoritative audited lists (mirror the Rust reference sets).
_BYTE_COMPILED = set(
    """
    set unset proc if while for foreach switch return break continue expr catch
    try throw package namespace upvar uplevel variable global append lappend incr
    info string list llength lindex lrange lsort lsearch lreplace linsert dict
    array regexp regsub scan format open close read gets eof flush seek tell
    fconfigure fcopy fileevent socket after update vwait rename source eval apply
    tailcall error cd file glob clock binary encoding interp load exit pid exec
    chan puts
    """.split()
)

_NOT_PROC_FACTORY = set(
    """
    proc namespace if switch while for foreach try catch eval apply expr uplevel
    upvar variable set lappend dict array string list lindex package source
    interp oo::class oo::define oo::objdefine
    """.split()
)

_FRAMELESS_RUNTIME = set(
    """
    list lindex lrange linsert llength lsort lsearch lappend lreverse lreplace
    lrepeat lassign concat split join string expr global variable upvar namespace
    set incr append unset puts return error continue break
    """.split()
)


def _load_all_dialects() -> None:
    for d in ("tcl8.6", "tcl9.0", "f5-irules", "f5-iapps"):
        REGISTRY._ensure_dialect_loaded(d)


def _names_with_trait(trait: str) -> set[str]:
    return {
        name
        for name, specs in REGISTRY.specs_by_name.items()
        if any(getattr(spec, trait, False) for spec in specs)
    }


def test_byte_compiled_covers_the_core_builtins():
    _load_all_dialects()
    assert _names_with_trait("byte_compiled") == _BYTE_COMPILED


def test_not_proc_factory_covers_registered_skip_heads():
    _load_all_dialects()
    assert _names_with_trait("not_proc_factory") == _NOT_PROC_FACTORY


def test_frameless_runtime_covers_the_audited_allow_list():
    _load_all_dialects()
    assert _names_with_trait("frameless_runtime") == _FRAMELESS_RUNTIME


def test_query_methods_agree_with_traits():
    _load_all_dialects()
    assert REGISTRY.is_byte_compiled("set") is True
    assert REGISTRY.is_byte_compiled("pwd") is False  # not byte-compiled
    assert REGISTRY.is_not_proc_factory("proc") is True
    assert REGISTRY.is_not_proc_factory("method") is False  # residual, not registry
    assert REGISTRY.is_frameless_runtime("list") is True
    assert REGISTRY.is_frameless_runtime("foreach") is False
    # Accepts canonical (``::``-prefixed) spellings too.
    assert REGISTRY.is_byte_compiled("::set") is True
