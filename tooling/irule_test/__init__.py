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

"""iRule test framework -- simulate BIG-IP TMM event processing.

This package provides a testing framework for F5 iRules that simulates
the BIG-IP TMM environment.  iRules run inside a real Tcl interpreter
with a TMM-like shim, while Python drives test configuration, SCF
loading, and assertion checking.

Quick start::

    from tooling.irule_test import IruleTestSession

    async with IruleTestSession(profiles=["TCP", "CLIENTSSL", "HTTP"]) as session:
        session.load_irule('''
            when HTTP_REQUEST {
                if { [HTTP::host] eq "api.example.com" } {
                    pool api_pool
                }
            }
        ''')
        session.add_pool("api_pool", ["10.0.1.1:80", "10.0.1.2:80"])
        result = await session.run_http_request(
            host="api.example.com",
            uri="/v1/health",
        )
        assert result.pool_selected == "api_pool"

For Tcl-only usage (no Python dependency)::

    source orchestrator.tcl
    ::orch::init
    ::orch::configure -profiles {TCP CLIENTSSL HTTP}
    ::orch::load_irule { ... }
    ::orch::add_pool api_pool {10.0.1.1:80 10.0.1.2:80}
    ::orch::run_http_request -host api.example.com -uri /v1/health
    ::orch::assert_pool_selected api_pool
    ::orch::summary
"""

from tooling.irule_test.bridge import IruleTestSession
from tooling.irule_test.topology import TopologyFromSCF

__all__ = ["IruleTestSession", "TopologyFromSCF"]
