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

"""Per-package registry for iRules command definitions."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_registry  # noqa: F401
from compiler.registry.namespace_models import EventRequires

_IRULES_ONLY = frozenset({"f5-irules"})

# Header names whose values typically contain secrets — shared by HTTP::header
# and HTTP::cookie insert/replace subcommands.
_SENSITIVE_HTTP_HEADERS = frozenset(
    {
        "authorization",
        "proxy-authorization",
        "x-api-key",
        "x-auth-token",
        "x-secret",
    }
)

# Shared EventRequires for LSN:: commands — available in client-side events
# where the virtual server is associated with an LSN pool/profile.
_LSN_EVENT_REQUIRES = EventRequires(
    profiles=frozenset({"LSN"}),
    also_in=frozenset(
        {
            "AUTH_RESULT",
            "AUTH_WANTCREDENTIAL",
            "CACHE_REQUEST",
            "CACHE_UPDATE",
            "CLIENTSSL_CLIENTCERT",
            "CLIENTSSL_HANDSHAKE",
            "CLIENT_ACCEPTED",
            "CLIENT_DATA",
            "HTTP_CLASS_FAILED",
            "HTTP_CLASS_SELECTED",
            "HTTP_REQUEST",
            "HTTP_REQUEST_DATA",
            "LB_SELECTED",
            "MR_INGRESS",
            "RTSP_REQUEST",
            "RTSP_REQUEST_DATA",
            "SIP_REQUEST",
            "STREAM_MATCHED",
        }
    ),
)

_REGISTRY, register = make_registry()
