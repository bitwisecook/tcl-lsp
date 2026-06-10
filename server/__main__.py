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
