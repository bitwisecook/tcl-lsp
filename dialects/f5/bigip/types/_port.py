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

"""Port + PortRange value types."""

from __future__ import annotations

from dataclasses import dataclass

# F5 port wildcards: ``"any"`` / ``"*"`` / ``0`` all mean "match any port".
# Stored canonically as port 0; rendered as ``"any"`` for round-trip clarity
# (matches the F5 convention used in destination strings).
_PORT_ANY_TOKENS = frozenset({"any", "*", "0"})


@dataclass(frozen=True, slots=True)
class Port:
    """A single TCP/UDP port (0-65535) or the wildcard ``any``.

    Port 0 has special semantics in F5: it's the wildcard "match any
    port".  We store wildcard as ``port=0`` and remember whether the
    user spelt it ``"any"`` / ``"*"`` / ``0`` so :meth:`__str__`
    round-trips the original spelling.
    """

    port: int  # 0..65535; 0 means wildcard
    spelling: str = ""  # original textual form ("80" / "any" / "*" / "0")

    @classmethod
    def parse(cls, text: str) -> "Port":
        """Parse *text* as a port number or wildcard token.

        Raises :class:`ValueError` on invalid input.
        """
        text = text.strip()
        if not text:
            raise ValueError("Port: empty input")
        lower = text.lower()
        if lower in _PORT_ANY_TOKENS:
            return cls(port=0, spelling=text)
        try:
            value = int(text, 10)
        except ValueError as exc:
            raise ValueError(f"Port: not a number ({text!r})") from exc
        if not (0 <= value <= 65535):
            raise ValueError(f"Port: out of range ({value})")
        return cls(port=value, spelling=text)

    @classmethod
    def try_parse(cls, text: str) -> "Port | None":
        try:
            return cls.parse(text)
        except (ValueError, TypeError):
            return None

    @property
    def is_any(self) -> bool:
        """``True`` when this port matches every port (wildcard)."""
        return self.port == 0

    def __str__(self) -> str:
        if self.spelling:
            return self.spelling
        if self.port == 0:
            return "any"
        return str(self.port)


@dataclass(frozen=True, slots=True)
class PortRange:
    """An inclusive port range (e.g. ``1024-65535``).

    Single-port ranges are accepted (``80-80``) but a bare port
    should use :class:`Port` directly.
    """

    low: int
    high: int

    @classmethod
    def parse(cls, text: str) -> "PortRange":
        """Parse a ``LOW-HIGH`` port range.

        Single ports without a hyphen raise :class:`ValueError`; use
        :class:`Port` for those.  Reverse ranges (``high < low``) are
        rejected.
        """
        text = text.strip()
        if "-" not in text:
            raise ValueError(f"PortRange: missing '-' in {text!r}")
        low_text, _, high_text = text.partition("-")
        try:
            low = int(low_text.strip(), 10)
            high = int(high_text.strip(), 10)
        except ValueError as exc:
            raise ValueError(f"PortRange: not numeric ({text!r})") from exc
        if not (0 <= low <= 65535 and 0 <= high <= 65535):
            raise ValueError(f"PortRange: out of range ({text!r})")
        if high < low:
            raise ValueError(f"PortRange: reverse range ({text!r})")
        return cls(low=low, high=high)

    @classmethod
    def try_parse(cls, text: str) -> "PortRange | None":
        try:
            return cls.parse(text)
        except (ValueError, TypeError):
            return None

    def __contains__(self, port: object) -> bool:
        if isinstance(port, Port):
            return self.low <= port.port <= self.high
        if isinstance(port, int):
            return self.low <= port <= self.high
        return False

    def __str__(self) -> str:
        return f"{self.low}-{self.high}"
