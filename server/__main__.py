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

"""Entry point: ``python -m server`` and the ``tcl-lsp`` console script.

Boots the LSP server over stdio.  The body that used to run at module
import is now inside ``main()`` so the `[project.scripts]` entry
``tcl-lsp = "server.__main__:main"`` resolves to a callable.
"""

import logging
import time


def main() -> None:
    process_start = time.monotonic()

    t0 = time.perf_counter()
    from .server import server

    import_ms = (time.perf_counter() - t0) * 1000
    logging.getLogger(__name__).info("[timing] server module import: %.0fms", import_ms)

    # Expose the startup timestamp so on_initialized can log total startup time.
    # Dynamic attribute on pygls' LanguageServer — read back by server.workspace_init.
    server._process_start_time = process_start  # type: ignore[attr-defined]  # ty: ignore[unresolved-attribute]

    server.start_io()  # type: ignore[attr-defined]  # ty: ignore[unresolved-attribute]


if __name__ == "__main__":
    main()
