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

"""Exception hierarchy for the ``tclpkg`` package manager.

These exceptions are raised by the library and propagate through the
CLI verb handlers.  They bubble up to ``tooling.tcl.main.main()`` where
the existing ``except Exception`` clause prints the message to stderr
and returns exit code 2 — the same behaviour as the surrounding ``tcl``
CLI verbs so users get a consistent experience.
"""

from __future__ import annotations


class TclPkgError(Exception):
    """Base class for every tclpkg-specific error.

    Subclasses set a more specific category and (optionally) a hint
    describing how the user can recover.  The CLI prints the exception
    message verbatim, so compose messages in the imperative, user-facing
    tone already used by other ``tcl`` verbs.
    """

    category: str = "tclpkg"

    def __init__(self, message: str, *, hint: str | None = None) -> None:
        super().__init__(message)
        self.message = message
        self.hint = hint

    def __str__(self) -> str:
        if self.hint:
            return f"{self.message}\n  hint: {self.hint}"
        return self.message


class ManifestError(TclPkgError):
    """The ``tclpkg.tcl`` manifest could not be parsed or is invalid."""

    category = "manifest"

    def __init__(
        self,
        message: str,
        *,
        path: str | None = None,
        line: int | None = None,
        hint: str | None = None,
    ) -> None:
        location = ""
        if path:
            location = path
            if line is not None:
                location += f":{line}"
            location += ": "
        super().__init__(f"{location}{message}", hint=hint)
        self.path = path
        self.line = line


class ResolutionError(TclPkgError):
    """MVS resolver could not produce a valid dependency graph.

    Includes the walk chain that led to the failure so ``tcl pkg why``-style
    diagnostics can be offered in the error message.
    """

    category = "resolver"

    def __init__(
        self,
        message: str,
        *,
        chain: list[str] | None = None,
        hint: str | None = None,
    ) -> None:
        if chain:
            path = "\n  via ".join(chain)
            message = f"{message}\n  via {path}"
        super().__init__(message, hint=hint)
        self.chain = list(chain) if chain else []


class IntegrityError(TclPkgError):
    """A cached package or lockfile entry failed its SHA-256 check."""

    category = "integrity"

    def __init__(
        self,
        message: str,
        *,
        expected: str | None = None,
        actual: str | None = None,
        hint: str | None = None,
    ) -> None:
        detail = ""
        if expected and actual:
            detail = f"\n  expected {expected}\n  got      {actual}"
        super().__init__(message + detail, hint=hint)
        self.expected = expected
        self.actual = actual


class RegistryError(TclPkgError):
    """The tcltk-pkgs registry could not be read or contacted."""

    category = "registry"
