"""CLI verb sub-package for the ``tcl`` command.

Each module in this package exposes an ``add_*_subparser()`` function that
registers a verb group (e.g. ``tcl pkg …``, ``tcl venv …``) onto the main
argparse subparser group in ``explorer.tcl_cli.parse_args()``.
"""
