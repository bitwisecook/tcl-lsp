"""Verb registry for the ``f5`` CLI.

Mirrors :mod:`explorer.verbs._registry` but keeps the f5 verb list in
its own module-global so ``f5`` and ``tcl`` / ``irule`` brief-help
output do not bleed into one another.  The two registries are
intentionally decoupled — refactoring one must not require changing
the other.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass, field
from typing import Callable


@dataclass
class _VerbSpec:
    name: str
    configure: Callable
    aliases: tuple[str, ...]
    help: str
    formatter_class: type = field(default_factory=lambda: argparse.HelpFormatter)


_VERB_REGISTRY: list[_VerbSpec] = []


def verb(
    name: str,
    *,
    aliases: tuple[str, ...] | list[str] = (),
    help: str = "",
    formatter_class: type = argparse.HelpFormatter,
) -> Callable:
    """Decorator: register a verb-configuration function in the f5 CLI."""

    def decorator(fn: Callable) -> Callable:
        _VERB_REGISTRY.append(
            _VerbSpec(
                name=name,
                configure=fn,
                aliases=tuple(aliases),
                help=help,
                formatter_class=formatter_class,
            )
        )
        return fn

    return decorator


def get_verb_catalogue() -> list[tuple[str, str, str]]:
    """Return ``(name, primary_alias, help)`` tuples for the brief help screen."""
    return [
        (spec.name, spec.aliases[0] if spec.aliases else "", spec.help) for spec in _VERB_REGISTRY
    ]


def apply_verb_registrations(
    sub: argparse._SubParsersAction,  # noqa: SLF001
    *,
    prog_name: str,
    default_dialect: str,
) -> None:
    """Add every registered f5 verb to *sub*."""
    for spec in _VERB_REGISTRY:
        p = sub.add_parser(
            spec.name,
            aliases=list(spec.aliases),
            help=spec.help,
            formatter_class=spec.formatter_class,
        )
        spec.configure(p, prog_name=prog_name, default_dialect=default_dialect)
