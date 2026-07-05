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

"""Tests for shared iRules event-context detection helper."""

from __future__ import annotations

from server.features.irules_context import find_enclosing_when_event


def test_find_enclosing_when_event_simple_block() -> None:
    source = "when CLIENT_DATA {\n    set payload [TCP::payload]\n}\n"
    event, anchor = find_enclosing_when_event(source, 1)
    assert event == "CLIENT_DATA"
    assert anchor == 0


def test_find_enclosing_when_event_prefers_innermost_nested_when() -> None:
    source = (
        'when CLIENT_ACCEPTED {\n    when HTTP_REQUEST {\n        log local0. "nested"\n    }\n}\n'
    )
    event, anchor = find_enclosing_when_event(source, 2)
    assert event == "HTTP_REQUEST"
    assert anchor == 1


def test_find_enclosing_when_event_returns_none_outside_events() -> None:
    source = "set x 1\nputs $x\n"
    event, anchor = find_enclosing_when_event(source, 1)
    assert event is None
    assert anchor == 1
