# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""CLI verb sub-package for the ``tcl`` command.

Two registration patterns are used:

- **``@verb`` decorator** (``_registry.py``): single-level verbs (opt, diag, lint,
  validate, format, minify, unminify-error, symbols, diagram, callgraph,
  symbolgraph, dataflow, event-order, event-info, command-info, highlight, dis,
  compwasm, diff, explore, convert, completion, help).  Each verb module calls
  ``apply_verb_registrations`` via ``load_verbs()`` below.

- **``add_*_subparser()``**: complex verb groups with sub-sub-commands (pkg,
  venv, docker).  These are imported directly in ``tcl_cli.parse_args``.
"""


def load_verbs() -> None:
    """Import all ``@verb``-decorated modules, triggering their registrations."""
    from . import (  # noqa: F401
        compile,
        completion,
        diag,
        diff,
        graphs,
        highlight,
        lookup,
        minimize,
        misc,
        transform,
    )
