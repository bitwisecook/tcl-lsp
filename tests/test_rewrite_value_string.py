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

"""Unit tests for ``_rewrite_value_string`` in ``compiler.inlining._rename``.

PR #237 review: backslash-protection rules for the rename walker.
"""

from __future__ import annotations

from compiler.inlining._rename import _rewrite_value_string


class TestRewriteValueStringEscape:
    def test_unescaped_dollar_substitutes(self):
        assert _rewrite_value_string("$x", {"x": "y"}) == "$y"

    def test_escaped_dollar_kept_literal(self):
        # ``\$x`` is the Tcl escape for a literal ``$``.  After
        # rewrite the ``$x`` portion must NOT be renamed because
        # the runtime won't substitute it either.
        assert _rewrite_value_string(r"\$x", {"x": "y"}) == r"\$x"

    def test_double_backslash_dollar_does_substitute(self):
        # ``\\$x`` — the two backslashes become a literal ``\`` and
        # ``$x`` is unescaped substitution.
        assert _rewrite_value_string(r"\\$x", {"x": "y"}) == r"\\$y"

    def test_brace_form_renames(self):
        assert _rewrite_value_string("${x}", {"x": "y"}) == "${y}"

    def test_array_element_renames_base_only(self):
        assert _rewrite_value_string("$arr(idx)", {"arr": "RA"}) == "$RA(idx)"

    def test_no_match_returns_original(self):
        s = "no $z here"
        assert _rewrite_value_string(s, {"x": "y"}) is s

    def test_unrenamed_var_kept(self):
        assert _rewrite_value_string("$other", {"x": "y"}) == "$other"
