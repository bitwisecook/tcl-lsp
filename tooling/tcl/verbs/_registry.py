"""Verb registry for the ``tcl`` CLI.

The ``@verb`` decorator registers a verb-configuration function into a
central list.  ``apply_verb_registrations`` iterates that list to add
every verb to an argparse subparser group in a single call.
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
    """Decorator: register a verb-configuration function in the CLI registry.

    The decorated function receives ``(parser, *, prog_name, default_dialect)``
    and is responsible for adding arguments and calling ``parser.set_defaults``.
    Dynamic per-verb description/epilog (which may reference ``prog_name``) can
    be set directly on the parser inside the decorated function.
    """

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
    """Add every registered verb to *sub*."""
    for spec in _VERB_REGISTRY:
        p = sub.add_parser(
            spec.name,
            aliases=list(spec.aliases),
            help=spec.help,
            formatter_class=spec.formatter_class,
        )
        spec.configure(p, prog_name=prog_name, default_dialect=default_dialect)
