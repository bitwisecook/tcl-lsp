"""Single-source-of-truth coverage for the iRules-dialect check.

The canonical predicate lives in ``compiler.registry.dialects`` and is
routed through by taint, the unused-procs optimiser, the minifier, and
the active-dialect convenience wrapper in ``runtime`` — so the legacy
``irules`` alias and any future spelling change live in one place.
"""

from __future__ import annotations

from compiler.registry.dialect import dialect_scope
from compiler.registry.dialects import IRULES_DIALECT_NAMES, is_irules_dialect
from compiler.registry.runtime import is_irules_dialect as active_is_irules


def test_value_predicate_accepts_canonical_and_legacy_alias():
    assert is_irules_dialect("f5-irules") is True
    assert is_irules_dialect("irules") is True


def test_value_predicate_rejects_other_dialects_and_none():
    assert is_irules_dialect("tcl8.6") is False
    assert is_irules_dialect("f5-iapps") is False
    assert is_irules_dialect("f5-bigip") is False
    assert is_irules_dialect(None) is False


def test_irules_dialect_names_membership():
    assert IRULES_DIALECT_NAMES == frozenset({"f5-irules", "irules"})


def test_active_dialect_wrapper_delegates_to_value_predicate():
    with dialect_scope(dialect="f5-irules"):
        assert active_is_irules() is True
    with dialect_scope(dialect="tcl8.6"):
        assert active_is_irules() is False
