"""Argcomplete integration for the tcl and f5 CLIs.

We use argcomplete to drive shell completion from the live argparse
parser instead of hand-maintaining per-shell scripts.  Two small twists
on top of the stock behaviour:

1.  **Hide aliases from the picker.**  Verbs like ``opt`` register
    aliases (``optimise``, ``optimize``).  argparse stores every alias
    as a key in ``_SubParsersAction.choices`` so it dispatches
    correctly, but we don't want the picker to show three entries that
    do the same thing.  Argparse keeps a separate ``_choices_actions``
    list with one entry per *primary* registration, which we use as
    the source of truth for what to show.
2.  **Graceful import fallback.**  Bare-Python invocations (during
    development, before ``pip install argcomplete``) must not break,
    so :func:`autocomplete` no-ops when the module is absent.
"""

from __future__ import annotations

import argparse


def autocomplete(parser: argparse.ArgumentParser) -> None:
    """Hook the argcomplete protocol onto *parser*.

    A no-op in normal CLI runs (the shell only sets the trigger
    env vars during a ``<TAB>`` completion).
    """
    try:
        from argcomplete.finders import CompletionFinder
    except ImportError:
        return

    class _AliasHidingFinder(CompletionFinder):
        """Filter out alias names from subverb completion."""

        def _get_subparser_completions(self, parser, cword_prefix):
            # Build the set of primary names from _choices_actions (one
            # entry per `add_parser` call — aliases never show up here).
            primary = {action.dest for action in parser._get_subactions()}  # noqa: SLF001
            for action in parser._get_subactions():  # noqa: SLF001
                if action.dest.startswith(cword_prefix):
                    self._display_completions[action.dest] = action.help or ""
            return sorted(name for name in primary if name.startswith(cword_prefix))

    _AliasHidingFinder()(parser)
