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

"""End-to-end tests that drive a real, packaged LSP server over JSON-RPC.

See ``conftest.py`` for the shared fixtures.  The design goal is one
long-lived server per session (per xdist worker) that many tests query — the
same surface an editor (or the Rust port) talks to, so this doubles as a
cross-implementation conformance suite.

Layout:

- ``harness.py`` — the ``LspServerClient`` JSON-RPC driver: lifecycle, raw
  ``request``/``notify``, per-feature helpers (``hover``, ``completion``,
  ``definition``, ``references``, ``rename``, ``code_actions``, …), incremental
  ``change_document`` / ``replace_document``, and version-targeted
  ``await_diagnostics`` for the server's push pipeline.
- ``_lsp_helpers.py`` — decoders for the raw JSON response shapes (Hover
  contents, Location vs LocationLink, hierarchical symbols, the semantic-token
  delta encoding).
- ``_textmirror.py`` — a faithful client-side mirror of LSP incremental edits
  (UTF-16 positions) used by the edit-tracking stress tests as an oracle.
- ``test_*_e2e.py`` — feature suites ported from the in-process
  ``tests/test_<feature>.py`` providers (hover, completion, definition,
  references, document symbols, signature help, document highlight, rename,
  semantic tokens, code actions, diagnostics, navigation, structure, commands)
  onto the live packaged server.
- ``test_edit_tracking_stress_e2e.py`` — hostile, heavy edit-cycle tests that
  punish the server's open-document buffer tracking.

To add a test, take the ``lsp_server`` fixture, mint a URI with ``uri_factory``,
``open_ready`` the document (which waits for the first analysis), and assert on
the response.
"""
