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

"""RouteDomain value type — the F5 ``%N`` route-domain suffix."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class RouteDomain:
    """An F5 route-domain identifier (the ``%N`` suffix on addresses).

    Route-domain 0 is the default (no segregation); higher numbers
    select specific routing tables on multi-tenant deployments.
    Stored as the integer ID; rendered as ``%N`` so it concatenates
    cleanly behind an address.
    """

    id: int

    @classmethod
    def parse(cls, text: str) -> "RouteDomain":
        """Parse *text* as a route-domain ID.

        Accepts ``"5"`` and ``"%5"`` and ``"   5  "``.  Negatives or
        non-integers raise :class:`ValueError`.
        """
        text = text.strip()
        if text.startswith("%"):
            text = text[1:]
        if not text:
            raise ValueError("RouteDomain: empty input")
        try:
            value = int(text, 10)
        except ValueError as exc:
            raise ValueError(f"RouteDomain: not numeric ({text!r})") from exc
        if value < 0:
            raise ValueError(f"RouteDomain: negative ({value})")
        return cls(id=value)

    @classmethod
    def try_parse(cls, text: str) -> "RouteDomain | None":
        try:
            return cls.parse(text)
        except (ValueError, TypeError):
            return None

    @property
    def is_default(self) -> bool:
        """``True`` when this is route-domain 0 (no segregation)."""
        return self.id == 0

    def __str__(self) -> str:
        return f"%{self.id}"
