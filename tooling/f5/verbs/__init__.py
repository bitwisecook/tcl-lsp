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

"""F5-specific CLI verbs for the ``f5`` command.

Verbs here use the verb registry in :mod:`tooling.f5.verbs._registry`,
which is independent of the ``tcl`` / ``irule`` registry under
:mod:`tooling.tcl.verbs._registry`.
"""


def load_verbs() -> None:
    """Import all ``@verb``-decorated f5 modules, triggering their registrations."""
    from . import (  # noqa: F401
        cleanup,
        completion,
        convert,
        diff,
        enrich_pcapng,
        enrich_wireshark,
        explain,
        explain_flow,
        extract,
        fetch,
        graph,
        grep,
        merge,
        pcap_remap,
        pull,
        push,
        query,
        redact,
        rename,
        secrets,
        split,
        stats,
        tmsh,
        unredact,
        validate,
    )
