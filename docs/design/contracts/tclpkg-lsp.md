# tclpkg LSP integration

## Symptom

The LSP server does not detect the tclpkg project root, venv library
paths are not picked up, or the "Install via tclpkg" code action does
not appear.

## Decision rules / contracts

1. Project root detection: on ``initialized``, walk up from
   ``workspace.root_path`` looking for ``tclpkg.tcl``.
2. If found, add ``<root>/lib/`` to the scanner's ``library_paths``.
3. Venv detection: check ``$TCL_VENV`` or ``.venv/tclvenv.cfg`` next to
   the project root. Add ``<venv>/lib/`` to ``library_paths``.
4. ``_KNOWN_TCL_LSP_SECTIONS`` includes ``"packageManager"`` so editor
   settings under ``tclLsp.packageManager.*`` flow through dot-path routing.
5. ``tcl-lsp.tclpkg.install`` command handler: triggered by the code
   action, accepts ``(package_name, uri)`` and returns a status dict.
6. ``tcl-lsp.tclpkg.search`` command handler: searches the offline
   registry cache and returns up to 20 results.
7. W120 code action now offers both "Add 'package require'" and
   "Install via tclpkg" quick-fixes.
8. W130–W134 diagnostic codes cover tclpkg-specific issues (lockfile
   drift, integrity mismatch, safe-mode violation, missing pkgIndex).
9. VS Code settings: ``registryUrl``, ``cacheDir``,
   ``autoInstallOnSave``, ``offline`` under the "Package Manager" group.

## File-path anchors

- ``lsp/server.py:424`` — ``_KNOWN_TCL_LSP_SECTIONS``
- ``lsp/server.py:3251`` — project root and venv detection in ``on_initialized``
- ``lsp/server.py:2007`` — ``tcl-lsp.tclpkg.install`` command handler
- ``lsp/features/code_actions.py:381`` — ``_tclpkg_install_action()``
- ``tclpkg/diagnostics.py`` — W130–W134 code registration
- ``editors/vscode/package.json`` — ``tclLsp.packageManager.*`` settings
