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

"""Enums shared across the BIG-IP data model."""

from __future__ import annotations

from enum import Enum, auto


class DataGroupType(Enum):
    """Whether a data-group is stored inline or in an external file."""

    INTERNAL = auto()
    EXTERNAL = auto()


class ProfileType(Enum):
    """Broad classification of BIG-IP profile types."""

    HTTP = auto()
    TCP = auto()
    UDP = auto()
    CLIENT_SSL = auto()
    SERVER_SSL = auto()
    FTP = auto()
    DNS = auto()
    SIP = auto()
    DIAMETER = auto()
    FIX = auto()
    RADIUS = auto()
    MQTT = auto()
    WEBSOCKET = auto()
    STREAM = auto()
    HTML = auto()
    REWRITE = auto()
    FASTHTTP = auto()
    FASTL4 = auto()
    ONE_CONNECT = auto()
    PERSISTENCE = auto()
    OTHER = auto()
