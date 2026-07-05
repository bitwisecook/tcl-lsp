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

"""F5 BIG-IP configuration file parsing and validation.

Parses ``bigip.conf`` / SCF files and cross-references iRules against
the configuration objects they use (data-groups, pools, profiles, etc.).

Public API
----------
parse_bigip_conf(source)
    Parse a configuration file and return a :class:`BigipConfig`.

validate_bigip_config(config)
    Run all cross-reference validations.

get_bigip_diagnostics(config)
    Validate and return LSP-ready diagnostics.
"""

from __future__ import annotations

from .model import BigipConfig
from .object_registry import (
    OBJECT_KIND_SPECS,
    build_bigip_object_registry,
    get_default_bigip_object_registry,
)
from .parser import parse_bigip_conf
from .validator import validate_bigip_config

__all__ = [
    "BigipConfig",
    "OBJECT_KIND_SPECS",
    "build_bigip_object_registry",
    "get_default_bigip_object_registry",
    "parse_bigip_conf",
    "validate_bigip_config",
]
