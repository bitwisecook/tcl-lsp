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

"""Tests for the f5 CLI verb registry, including the ``irule`` sub-verbs."""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from tooling.f5.main import main


def _run(args: list[str], capsys) -> tuple[int, str, str]:
    code = main(args)
    captured = capsys.readouterr()
    return code, captured.out, captured.err


def test_f5_irule_event_order_json_reports_events(capsys):
    source = "when HTTP_RESPONSE { return }\nwhen HTTP_REQUEST { return }\n"

    code, out, err = _run(
        ["irule", "event-order", "--source", source, "--json"],
        capsys,
    )

    assert code == 0
    payload = json.loads(out)
    assert payload["count"] == 2
    assert payload["dialect"] == "f5-irules"
    names = [item["name"] for item in payload["events"]]
    assert "HTTP_REQUEST" in names
    assert "HTTP_RESPONSE" in names
    assert err == ""


def test_f5_irule_event_info_json_known_event(capsys):
    code, out, err = _run(["irule", "event-info", "HTTP_REQUEST", "--json"], capsys)

    assert code == 0
    payload = json.loads(out)
    assert payload["event"] == "HTTP_REQUEST"
    assert payload["known"] is True
    assert payload["validCommandCount"] >= 1
    assert err == ""


def test_f5_irule_event_info_dual_transport_event(capsys):
    code, out, err = _run(["irule", "event-info", "CLIENT_ACCEPTED", "--json"], capsys)

    assert code == 0
    payload = json.loads(out)
    assert payload["transport"] == "tcp/udp"
    assert err == ""


def test_f5_irule_event_order_alias_works(capsys):
    source = "when HTTP_REQUEST { return }\n"

    code, out, _err = _run(
        ["irule", "eventorder", "--source", source, "--json"],
        capsys,
    )

    assert code == 0
    payload = json.loads(out)
    assert payload["count"] == 1


def test_f5_irule_requires_a_subaction(capsys):
    import pytest

    with pytest.raises(SystemExit):
        main(["irule"])

    err = capsys.readouterr().err
    assert "irule_action" in err or "required" in err
