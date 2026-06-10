"""User-defined TclOO / snit class-name extraction from IR.

A leaf helper lifted out of :mod:`compiler.compilation_unit` so that
:mod:`compiler.core_analyses` can extract class names without importing
``compilation_unit`` — which previously closed a
``compilation_unit`` ↔ ``core_analyses`` ↔ ``interprocedural`` import cycle.
This module depends only on the IR, the command registry, and name
normalisation, so it sits below both analysis modules in the import graph.
"""

from __future__ import annotations

from shared.naming import normalise_qualified_name

from .ir import IRBarrier, IRBlock, IRCall, IRModule, IRStatement
from .registry import REGISTRY

_oo_metaclass_cache: frozenset[str] | None = None


def _oo_metaclasses() -> frozenset[str]:
    global _oo_metaclass_cache
    if _oo_metaclass_cache is None:
        _oo_metaclass_cache = REGISTRY.check_trait_commands("is_oo_metaclass")
    return _oo_metaclass_cache


# snit type-definers (tcllib).  ``snit::type Name body`` makes ``Name`` a
# class whose instances are created via ``Name create x`` / ``Name %AUTO%`` /
# (widgets) ``Name .path`` — recognised in ``_return_type_for_command`` so the
# created object's variable is typed ``OBJECT`` (suppresses W307 dispatch FPs).
_SNIT_DEFINERS: frozenset[str] = frozenset(
    {
        "snit::type",
        "snit::widget",
        "snit::widgetadaptor",
        "::snit::type",
        "::snit::widget",
        "::snit::widgetadaptor",
    }
)


def extract_class_names(ir_module: IRModule) -> frozenset[str]:
    """Extract user-defined TclOO / snit class names from IR statements.

    Scans ``oo::class create ClassName`` (and similar metaclass) patterns plus
    ``snit::type Name`` / ``snit::widget`` / ``snit::widgetadaptor`` in the
    top-level script and procedure bodies.
    """
    names: set[str] = set()

    def _qualify(name: str, namespace: str) -> str:
        # Absolute names live at the global root; relative names resolve against
        # the enclosing namespace (mirrors tclsh ``create`` and the analyser's
        # ``_qualify_oo_name``).  normalise collapses any doubled ``::``.
        if name.startswith("::"):
            return normalise_qualified_name(name)
        return normalise_qualified_name(f"{namespace}::{name}")

    def _scan(stmts: tuple[IRStatement, ...], namespace: str = "::") -> None:
        for stmt in stmts:
            if isinstance(stmt, IRBlock):
                # ``namespace eval ns {…}`` — recurse with the block's namespace
                # so a relative ``oo::class create Foo`` inside it is recorded as
                # ``::ns::Foo`` (was ``::Foo``, so a relative ``[Foo new]`` in the
                # namespace never matched → W307 instead of object typing).
                _scan(stmt.body.statements, stmt.namespace or namespace)
                continue
            cmd: str = ""
            args: tuple[str, ...] = ()
            if isinstance(stmt, (IRCall, IRBarrier)):
                cmd, args = stmt.command, stmt.args
            if (
                cmd in _oo_metaclasses()
                and len(args) >= 2
                and args[0] in ("create", "createWithNamespace")
            ):
                names.add(_qualify(args[1], namespace))
            elif cmd in _SNIT_DEFINERS and args:
                names.add(_qualify(args[0], namespace))

    _scan(ir_module.top_level.statements)
    for qname, proc in ir_module.procedures.items():
        # A class created in a proc body is named relative to the proc's own
        # namespace (``proc ::ns::p {} { oo::class create C }`` ⇒ ``::ns::C``).
        proc_ns = normalise_qualified_name(qname).rsplit("::", 1)[0] or "::"
        _scan(proc.body.statements, proc_ns)
    return frozenset(names)
