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

"""Registry-generated subcommand behaviour, asserted through ``tcl diag``.

For every ensemble command (one with subcommands) in every Tcl dialect,
the front-end must report a bare call as "requires a subcommand", a
bogus subcommand as unknown, and must not flag a real subcommand.
"""

from __future__ import annotations

import pytest

from tooling.registry_snapshot import ALL_DIALECTS

from ._generators import subcommand_cases


@pytest.mark.parametrize("dialect", ALL_DIALECTS)
def test_generated_subcommand_diagnostics(dialect: str) -> None:
    cases = list(subcommand_cases(dialect))
    assert cases, f"no subcommand cases generated for {dialect}"
    failures = [msg for case in cases if (msg := case.check()) is not None]
    assert not failures, (
        f"{dialect}: {len(failures)}/{len(cases)} subcommand cases failed:\n"
        + "\n".join(failures[:40])
    )
