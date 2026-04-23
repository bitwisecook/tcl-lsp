"""CLI verb sub-package for the ``tcl`` command.

Two registration patterns are used:

- **``@verb`` decorator** (``_registry.py``): single-level verbs (opt, diag, lint,
  validate, format, minify, unminify-error, symbols, diagram, callgraph,
  symbolgraph, dataflow, event-order, event-info, command-info, highlight, dis,
  compwasm, diff, explore, convert, help).  Each verb module calls
  ``apply_verb_registrations`` via ``load_verbs()`` below.

- **``add_*_subparser()``**: complex verb groups with sub-sub-commands (pkg,
  venv, docker).  These are imported directly in ``tcl_cli.parse_args``.
"""


def load_verbs() -> None:
    """Import all ``@verb``-decorated modules, triggering their registrations."""
    from . import compile, diag, diff, graphs, highlight, lookup, misc, transform  # noqa: F401
