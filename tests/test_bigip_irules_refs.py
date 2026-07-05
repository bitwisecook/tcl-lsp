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

"""Tests for parser-backed iRules object reference extraction."""

from __future__ import annotations

from dialects.f5.bigip.irules_refs import extract_irules_object_references


def test_extract_irules_refs_for_pool_snatpool_and_datagroup() -> None:
    source = """
when HTTP_REQUEST {
    if {[class match -- [HTTP::host] equals /Common/host_dg]} {
        snatpool /Common/sp1
        pool /Common/web_pool
    }
}
"""
    refs = extract_irules_object_references(source)
    by_name = {(ref.command, ref.name): ref.kinds for ref in refs}

    assert ("class", "/Common/host_dg") in by_name
    assert ("snatpool", "/Common/sp1") in by_name
    assert ("pool", "/Common/web_pool") in by_name


def test_extract_irules_refs_nested_in_body_and_command_substitution() -> None:
    source = """
when HTTP_REQUEST {
    set count [active_members /Common/app_pool]
    LB::reselect pool /Common/fallback_pool
}
"""
    refs = extract_irules_object_references(source)
    names = [ref.name for ref in refs]
    assert "/Common/app_pool" in names
    assert "/Common/fallback_pool" in names
