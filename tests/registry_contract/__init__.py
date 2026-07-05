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

"""Front-end-behaviour contract tests for the command and graph registries.

These tests drive the real ``tcl`` and ``f5`` front-ends — and, for
presence and structural invariants, read the registry directly — asserting
behaviour against the language-agnostic golden CSVs under
``tests/baselines/registry/``.  The fixtures are the registry shape
contract; a Rust front-end re-implementing the ``command-info`` /
``event-info`` verbs is validated against the same files.
"""
