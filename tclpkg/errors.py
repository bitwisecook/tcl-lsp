"""Exception hierarchy for the ``tclpkg`` package manager.

These exceptions are raised by the library and propagate through the
CLI verb handlers.  They bubble up to ``explorer.tcl_cli.main()`` where
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
