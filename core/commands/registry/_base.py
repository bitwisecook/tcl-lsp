"""Shared base class and helpers for class-per-command definitions.

Each dialect package (``tcl``, ``irules``, ``iapps``) re-exports
:class:`CommandDef` from here and maintains its own ``_REGISTRY`` list
and ``register`` decorator.
"""

from __future__ import annotations

from collections.abc import Callable
from typing import TYPE_CHECKING, Protocol

from .models import ArgumentValueSpec, HoverSnippet

if TYPE_CHECKING:
    from .models import CommandSpec
    from .taint_hints import TaintHint


class _AvFactory(Protocol):
    """Callable protocol for the ``_av`` factory returned by :func:`make_av`."""

    def __call__(self, value: str, detail: str, synopsis: str = ...) -> ArgumentValueSpec: ...


def make_av(source: str) -> _AvFactory:
    """Return an ``_av`` factory bound to a specific documentation *source*.

    Usage at module level::

        _av = make_av(_SOURCE)
    """

    def _av(value: str, detail: str, synopsis: str = "") -> ArgumentValueSpec:
        return ArgumentValueSpec(
            value=value,
            detail=detail,
            hover=HoverSnippet(
                summary=detail,
                synopsis=(synopsis,) if synopsis else (),
                source=source,
            ),
        )

    return _av


class CommandDef:
    """Base class for a curated command definition.

    Subclasses must set ``name`` and implement :meth:`spec`.
    Override :meth:`role_hints` to co-locate ``ArgRole`` signatures
    alongside the command metadata.
    """

    name: str

    @classmethod
    def spec(cls) -> CommandSpec:
        raise NotImplementedError

    @classmethod
    def taint_hints(cls) -> TaintHint | None:
        """Return taint metadata for this command.

        Returning ``None`` means no taint information is available
        for this command.
        """
        return None


_RegisterFn = Callable[[type[CommandDef]], type[CommandDef]]


def make_registry() -> tuple[list[type[CommandDef]], _RegisterFn]:
    """Create a ``(_REGISTRY, register)`` pair for a dialect sub-package.

    Each sub-package calls this once at module level::

        _REGISTRY, register = make_registry()
    """
    registry: list[type[CommandDef]] = []

    def register(cls: type[CommandDef]) -> type[CommandDef]:
        """Class decorator that registers a command definition at import time."""
        registry.append(cls)
        return cls

    return registry, register
