"""Semantic model for Tcl source analysis.

Dataclasses representing the structural understanding of a Tcl file:
procedures, variables, scopes, namespaces, and diagnostics.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum, auto

from ..parsing.tokens import SourcePosition


# Source ranges
@dataclass(frozen=True, slots=True)
class Range:
    """A span in source text."""

    start: SourcePosition
    end: SourcePosition

    @classmethod
    def zero(cls) -> Range:
        """Return a zero-length range at position (0, 0, 0)."""
        pos = SourcePosition(line=0, character=0, offset=0)
        return cls(start=pos, end=pos)


# Diagnostic severity
class Severity(Enum):
    ERROR = auto()
    WARNING = auto()
    INFO = auto()
    HINT = auto()


# Diagnostic
@dataclass(frozen=True, slots=True)
class CodeFix:
    """A suggested fix for a diagnostic -- maps to an LSP TextEdit."""

    range: Range
    new_text: str
    description: str = ""


@dataclass(frozen=True, slots=True)
class Diagnostic:
    """A single error, warning, or info message attached to a source range."""

    range: Range
    message: str
    severity: Severity = Severity.ERROR
    code: str = ""
    fixes: tuple[CodeFix, ...] = ()


# Variable definition
@dataclass
class VarDef:
    """A variable known to the analyser."""

    name: str
    definition_range: Range
    references: list[Range] = field(default_factory=list)
    warn_if_unused: bool = False


# Procedure parameter
@dataclass(frozen=True, slots=True)
class ParamDef:
    """A parameter in a proc definition."""

    name: str
    has_default: bool = False
    default_value: str = ""


# Procedure definition
@dataclass
class ProcDef:
    """A procedure defined via 'proc'."""

    name: str
    qualified_name: str  # e.g. "::math::add"
    params: list[ParamDef]
    name_range: Range
    body_range: Range
    doc: str = ""  # extracted from preceding comment


# Scope
@dataclass
class Scope:
    """A lexical scope (global, namespace, or proc body)."""

    kind: str  # "global", "namespace", "proc"
    name: str  # scope identifier
    parent: Scope | None = None
    body_range: Range | None = None
    variables: dict[str, VarDef] = field(default_factory=dict)
    procs: dict[str, ProcDef] = field(default_factory=dict)
    children: list[Scope] = field(default_factory=list)


# Regex pattern occurrence
@dataclass(frozen=True, slots=True)
class RegexPattern:
    """A source range known to contain a regular expression pattern.

    Recorded by the analyser for every pattern argument of ``regexp``,
    ``regsub``, ``switch -regexp`` patterns, and (in the future) for
    variables whose value flows into one of those positions.
    """

    range: Range
    pattern: str  # the literal text of the pattern
    command: str  # originating command: "regexp", "regsub", "switch"


@dataclass(frozen=True, slots=True)
class CommandInvocation:
    """A command word observed during analysis."""

    name: str
    range: Range
    resolved_qualified_name: str | None = None


@dataclass(frozen=True, slots=True)
class PackageRequire:
    """A ``package require`` invocation observed during analysis."""

    name: str
    version: str | None
    range: Range
    conditional: bool = False  # True if inside if/catch/try guard


@dataclass(frozen=True, slots=True)
class PackageProvide:
    """A ``package provide`` invocation observed during analysis."""

    name: str
    version: str | None
    range: Range


@dataclass(frozen=True, slots=True)
class PackageContext:
    """Packages active in a file, with confidence levels.

    Built from ``AnalysisResult.package_requires`` and related signals.
    Used by the registry to filter command visibility and by diagnostics
    to control severity.
    """

    definite: frozenset[str]  # unconditional ``package require``
    probable: frozenset[str]  # conditional requires (if/catch guards)
    provided: frozenset[str]  # ``package provide`` in this file
    unknown_providers: bool  # True if ``load``, ``auto_path``, or dynamic require detected

    @property
    def all_required(self) -> frozenset[str]:
        """All required packages (definite + probable)."""
        return self.definite | self.probable

    def confidence(self, package: str) -> Confidence:
        """Return the detection confidence for *package*."""
        if package in self.definite:
            return Confidence.DEFINITE
        if package in self.probable:
            return Confidence.PROBABLE
        return Confidence.UNKNOWN


class Confidence(Enum):
    """Diagnostic confidence level based on package detection certainty."""

    DEFINITE = auto()  # unconditional package require
    PROBABLE = auto()  # conditional require (if/catch guard)
    UNKNOWN = auto()  # dynamic provider detected


# Analysis result
@dataclass
class AnalysisResult:
    """Complete analysis result for a single document."""

    global_scope: Scope = field(default_factory=lambda: Scope(kind="global", name="::"))
    all_procs: dict[str, ProcDef] = field(default_factory=dict)
    all_variables: dict[str, VarDef] = field(default_factory=dict)
    diagnostics: list[Diagnostic] = field(default_factory=list)
    # Inline suppression: line -> set of suppressed diagnostic codes.
    # Populated from ``# tcl-lsp: disable=CODE,...`` comment directives.
    suppressed_lines: dict[int, frozenset[str]] = field(default_factory=dict)
    regex_patterns: list[RegexPattern] = field(default_factory=list)
    command_invocations: list[CommandInvocation] = field(default_factory=list)
    package_requires: list[PackageRequire] = field(default_factory=list)
    package_provides: list[PackageProvide] = field(default_factory=list)
    has_dynamic_providers: bool = False  # True if load/auto_path detected

    def active_package_names(self) -> frozenset[str]:
        """Return the set of package names imported via ``package require``."""
        return frozenset(pr.name for pr in self.package_requires)

    def package_context(self) -> PackageContext:
        """Build a ``PackageContext`` from the analysis result."""
        definite: set[str] = set()
        probable: set[str] = set()
        for pr in self.package_requires:
            if pr.conditional:
                probable.add(pr.name)
            else:
                definite.add(pr.name)
        # Unconditional overrides conditional for same package.
        probable -= definite
        return PackageContext(
            definite=frozenset(definite),
            probable=frozenset(probable),
            provided=frozenset(pp.name for pp in self.package_provides),
            unknown_providers=self.has_dynamic_providers,
        )


@dataclass(frozen=True, slots=True)
class DocumentContext:
    """Computed once per document update, shared across all LSP features.

    Encapsulates all context facts (dialect, packages, Tk mode, event)
    needed by completion, diagnostics, hover, and code actions.  Avoids
    redundant re-computation across features within a single update cycle.
    """

    dialect: str | None = None
    active_packages: frozenset[str] = frozenset()
    tk_mode: str = "unknown"  # "enabled", "disabled", "unknown"
    package_context: PackageContext | None = None
    current_event: str | None = None
