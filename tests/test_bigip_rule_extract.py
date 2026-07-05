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

"""Regression tests for BIG-IP embedded iRule extraction ranges."""

from __future__ import annotations

from dialects.f5.bigip.rule_extract import find_embedded_rules, find_rule_at_offset


def test_find_embedded_rules_range_maps_to_braces() -> None:
    source = (
        "ltm rule /Common/my_irule {\n"
        "    when HTTP_REQUEST {\n"
        "        pool /Common/web_pool\n"
        "    }\n"
        "}\n"
    )
    rules = find_embedded_rules(source)
    assert len(rules) == 1
    rule = rules[0]

    assert rule.range.start.line == 0
    assert rule.range.start.character == 0
    assert source[rule.range.start.offset : rule.range.start.offset + 8] == "ltm rule"

    assert source[rule.range.end.offset] == "}"
    assert rule.range.end.line == 4
    assert rule.range.end.character == 0


def test_find_rule_at_offset_inside_block() -> None:
    source = 'ltm rule /Common/a {\n    when HTTP_REQUEST { log local0. "a" }\n}\n'
    offset = source.index("HTTP_REQUEST")
    rule = find_rule_at_offset(source, offset)
    assert rule is not None
    assert rule.full_path == "/Common/a"
