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

"""Tests for ai.shared.irule_analysis shared utilities."""

from __future__ import annotations


class TestEventMultiplicity:
    def test_rule_init(self) -> None:
        from ai.shared.irule_analysis import event_multiplicity

        assert event_multiplicity("RULE_INIT") == "init"

    def test_once_per_connection(self) -> None:
        from ai.shared.irule_analysis import event_multiplicity

        assert event_multiplicity("CLIENT_ACCEPTED") == "once_per_connection"

    def test_per_request(self) -> None:
        from ai.shared.irule_analysis import event_multiplicity

        assert event_multiplicity("HTTP_REQUEST") == "per_request"

    def test_unknown(self) -> None:
        from ai.shared.irule_analysis import event_multiplicity

        assert event_multiplicity("NOT_A_REAL_EVENT") == "unknown"


class TestOrderedEvents:
    def test_simple_irule(self) -> None:
        from ai.shared.irule_analysis import ordered_events

        source = """
when HTTP_REQUEST {
    pool web_pool
}
when HTTP_RESPONSE {
    HTTP::header remove Server
}
"""
        events = ordered_events(source)
        assert len(events) == 2
        assert events[0].name == "HTTP_REQUEST"
        assert events[0].index == 1
        assert events[0].multiplicity == "per_request"
        assert events[1].name == "HTTP_RESPONSE"
        assert events[1].index == 2

    def test_as_dicts(self) -> None:
        from ai.shared.irule_analysis import ordered_events_as_dicts

        source = "when RULE_INIT { set static::debug 0 }"
        dicts = ordered_events_as_dicts(source)
        assert len(dicts) == 1
        assert dicts[0] == {"index": 1, "name": "RULE_INIT", "multiplicity": "init"}

    def test_empty_source(self) -> None:
        from ai.shared.irule_analysis import ordered_events

        assert ordered_events("") == []

    def test_canonical_order(self) -> None:
        """Events are returned in firing order, not source order."""
        from ai.shared.irule_analysis import ordered_events

        source = """
when HTTP_RESPONSE { }
when CLIENT_ACCEPTED { }
when HTTP_REQUEST { }
"""
        events = ordered_events(source)
        names = [e.name for e in events]
        assert names == ["CLIENT_ACCEPTED", "HTTP_REQUEST", "HTTP_RESPONSE"]


class TestTaintWarnings:
    def test_empty_source(self) -> None:
        from ai.shared.irule_analysis import taint_warnings

        assert taint_warnings("") == []

    def test_returns_list(self) -> None:
        from ai.shared.irule_analysis import taint_warnings

        # Even for valid source with no taint issues, returns a list
        result = taint_warnings("when RULE_INIT { set static::x 1 }")
        assert isinstance(result, list)


class TestDiagramData:
    def test_empty_source(self) -> None:
        from ai.shared.irule_analysis import diagram_data

        data = diagram_data("")
        assert isinstance(data, dict)

    def test_simple_irule(self) -> None:
        from ai.shared.irule_analysis import diagram_data

        source = """
when HTTP_REQUEST {
    pool web_pool
}
"""
        data = diagram_data(source)
        assert isinstance(data, dict)
        # Should have events key
        assert "events" in data


class TestRangeToDict:
    def test_converts_range(self) -> None:
        from unittest.mock import MagicMock

        from ai.shared.irule_analysis import range_to_dict

        r = MagicMock()
        r.start.line = 1
        r.start.character = 5
        r.end.line = 1
        r.end.character = 10

        result = range_to_dict(r)
        assert result == {
            "start": {"line": 1, "character": 5},
            "end": {"line": 1, "character": 10},
        }
