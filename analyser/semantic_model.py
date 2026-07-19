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

"""Semantic model for Tcl source analysis.

Dataclasses representing the structural understanding of a Tcl file:
procedures, variables, scopes, namespaces, and diagnostics.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum, auto

from compiler.registry.stub_types import (  # noqa: F401  re-export for back-compat
    StubArgDef,
    StubCommandDef,
    StubExprDef,
)
from shared.diagnostic import (  # noqa: F401  re-export for back-compat; new code: import from shared.diagnostic
    CodeFix,
    Diagnostic,
    Range,
    Severity,
)
from shared.proc_traits import ProcArgTrait  # noqa: F401  re-export for back-compat
from shared.tokens import SourcePosition  # noqa: F401  re-export for back-compat


# Variable definition
@dataclass
class VarDef:
    """A variable known to the analyser."""

    name: str
    definition_range: Range
    references: list[Range] = field(default_factory=list)
    warn_if_unused: bool = False
    # Element names observed for array variables, e.g. ``set arr(foo) 1`` /
    # ``puts $arr(bar)`` populate ``{"foo", "bar"}``.  Empty for scalar vars.
    array_indices: set[str] = field(default_factory=set)


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
    # Per-parameter traits inferred from body analysis.
    # Maps parameter name to the set of traits detected.
    param_traits: dict[str, frozenset[ProcArgTrait]] = field(default_factory=dict)
    # True when this proc is registered as a ``trace`` callback.  Tcl's
    # trace API requires the callback to accept a fixed trailing argument
    # signature (e.g. ``name1 name2 op``); the body legitimately may not
    # use those arguments, so W214 is suppressed for the whole proc.
    is_trace_callback: bool = False


# OO method definition
@dataclass(frozen=True, slots=True)
class MethodDef:
    """A method defined within a TclOO class."""

    name: str
    params: tuple[ParamDef, ...]
    name_range: Range
    body_range: Range
    visibility: str = "public"  # "public" | "private" | "unexported"
    kind: str = "method"  # "method" | "classmethod" | "forward" | "constructor" | "destructor"
    doc: str = ""
    param_traits: dict[str, frozenset[ProcArgTrait]] = field(default_factory=dict)


# OO property definition
@dataclass(frozen=True, slots=True)
class PropertyDef:
    """A configurable property on a TclOO class."""

    name: str
    name_range: Range
    kind: str = "readwrite"  # "readable" | "writable" | "readwrite"
    has_getter: bool = False
    has_setter: bool = False


# OO class definition
@dataclass
class ClassDef:
    """A TclOO class extracted from source analysis."""

    name: str  # simple name
    qualified_name: str  # e.g. "::shapes::Point"
    name_range: Range  # range of class name in source
    body_range: Range  # range of definition body
    metaclass: str = "oo::class"  # "oo::class"|"oo::configurable"|"oo::abstract"|"oo::singleton"
    superclasses: list[str] = field(default_factory=list)
    mixins: list[str] = field(default_factory=list)
    # (name, range) for each superclass/mixin reference token, so a class's
    # find-references / rename spans its use in superclass/mixin declarations.
    superclass_refs: list[tuple[str, Range]] = field(default_factory=list)
    mixin_refs: list[tuple[str, Range]] = field(default_factory=list)
    methods: dict[str, MethodDef] = field(default_factory=dict)
    class_methods: dict[str, MethodDef] = field(default_factory=dict)
    constructors: list[MethodDef] = field(default_factory=list)
    destructor: MethodDef | None = None
    variables: list[str] = field(default_factory=list)
    properties: dict[str, PropertyDef] = field(default_factory=dict)
    filters: list[str] = field(default_factory=list)
    exports: set[str] = field(default_factory=set)
    unexports: set[str] = field(default_factory=set)
    doc: str = ""

    def copy(self) -> ClassDef:
        """Independent copy for analyser snapshots.

        ``ClassDef`` is mutable — a later ``oo::define``/``oo::objdefine`` adds
        entries to ``methods``/``class_methods``/… — so a snapshot must copy the
        mutable containers, or a mutation after the snapshot (or in a dirty
        chunk after restore) leaks into it and silences/raises W308 wrongly.
        The contained ``MethodDef``/``PropertyDef`` values are added, never
        mutated in place, so sharing them by reference is sound.
        """
        return ClassDef(
            name=self.name,
            qualified_name=self.qualified_name,
            name_range=self.name_range,
            body_range=self.body_range,
            metaclass=self.metaclass,
            superclasses=self.superclasses[:],
            mixins=self.mixins[:],
            superclass_refs=self.superclass_refs[:],
            mixin_refs=self.mixin_refs[:],
            methods=dict(self.methods),
            class_methods=dict(self.class_methods),
            constructors=self.constructors[:],
            destructor=self.destructor,
            variables=self.variables[:],
            properties=dict(self.properties),
            filters=self.filters[:],
            exports=set(self.exports),
            unexports=set(self.unexports),
            doc=self.doc,
        )


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
    classes: dict[str, ClassDef] = field(default_factory=dict)
    children: list[Scope] = field(default_factory=list)

    def _copy_tree(
        self,
        parent: Scope | None = None,
        class_map: dict[int, ClassDef] | None = None,
    ) -> Scope:
        """Fast recursive copy of the scope tree.

        Much faster than ``copy.deepcopy`` because it avoids cycle
        detection (``parent`` references create cycles that force
        deepcopy to maintain a memo dict), generic dispatch overhead,
        and copies frozen/immutable objects by reference.

        *class_map* maps a live ``ClassDef``'s ``id()`` to its already-made
        snapshot copy, so a scope's ``classes`` entry reuses the *same* copy as
        ``AnalysisResult.all_classes`` (preserving the live
        ``scope.classes[x] is all_classes[y]`` identity); a scope-local class
        not in the map is copied directly.  ``ClassDef`` is mutable, so it must
        be copied (the old shallow ``dict(self.classes)`` shared it).
        """
        new = Scope(
            kind=self.kind,
            name=self.name,
            parent=parent,
            body_range=self.body_range,  # frozen — shared reference
            variables={
                k: VarDef(
                    v.name,
                    v.definition_range,
                    list(v.references),
                    v.warn_if_unused,
                    set(v.array_indices),
                )
                for k, v in self.variables.items()
            },
            procs={
                k: ProcDef(
                    v.name,
                    v.qualified_name,
                    list(v.params),
                    v.name_range,
                    v.body_range,
                    v.doc,
                    dict(v.param_traits),
                    v.is_trace_callback,
                )
                for k, v in self.procs.items()
            },
            classes={
                k: ((class_map.get(id(v)) if class_map else None) or v.copy())
                for k, v in self.classes.items()
            },
        )
        new.children = [
            child._copy_tree(parent=new, class_map=class_map) for child in self.children
        ]
        return new


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
    # The ``::``-qualified namespace the call sits in (e.g. ``::`` or
    # ``::a``). Lets reference resolution scope an *unresolved* bare call
    # (a forward or cross-file reference) to the proc Tcl would pick —
    # current namespace, then global — instead of every same-named proc.
    enclosing_namespace: str | None = None


@dataclass(frozen=True, slots=True)
class MethodInvocation:
    """A TclOO method dispatch site observed during analysis.

    Covers the three dispatch forms whose command word is *not* the method
    name — ``$obj method args`` (object variable), ``my method args`` /
    ``self ... method`` (self-dispatch inside a method body), and
    ``[cmd] method args`` (object from a command result).  These never appear
    in :class:`CommandInvocation` under the method name, so find-references and
    the reference code-lens need this dedicated record to credit a method
    definition with its call sites (#956, #957).
    """

    method_name: str
    range: Range  # the method-name token span
    receiver: str  # "obj" | "my" | "self" | "cmd"
    # Qualified class name the call dispatches to, when resolvable — the class
    # of ``$obj`` (from ``set obj [Class new]``) or the enclosing class for a
    # ``my`` / ``self`` call.  ``None`` when the receiver type is unknown.
    class_name: str | None = None


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
class SourceTarget:
    """A ``source`` command target observed during analysis."""

    raw_path: str  # literal path text (may contain substitutions)
    range: Range
    is_literal: bool  # False if path contains $ or [ substitutions


@dataclass(frozen=True, slots=True)
class NamespaceImport:
    """A ``namespace import`` declaration observed during analysis.

    ``ns`` is the importing namespace (``::`` at top level).  ``pattern``
    is the fully-qualified import pattern, e.g. ``::term::ansi::send::*``
    or ``::some::ns::specific_cmd``.  When Tcl runs ``namespace import``
    it creates a local alias for every command in the source namespace
    matching the pattern; the LSP records the declaration so qualified
    references like ``vt::showat`` can be rewritten to the underlying
    fully-qualified name.
    """

    ns: str  # importing namespace, e.g. "::" or "::vt"
    pattern: str  # fully-qualified pattern, e.g. "::term::ansi::send::*"
    range: Range
    conjectured: bool = False
    """True when inferred from a tcllib-style ``X::import`` wrapper call
    rather than a direct ``namespace import`` — lower confidence, used
    only as a fallback."""


@dataclass(frozen=True, slots=True)
class AutoPathEntry:
    """A raw ``lappend auto_path`` / ``set auto_path`` argument.

    The extraction pass records every path element as-written, without
    attempting to evaluate it — resolution requires the document's file
    path (for ``[info script]``) and happens later in the LSP server
    via :func:`analyser.auto_path_eval.evaluate_auto_path_expr`.

    ``resolved_path`` is reserved for callers that want to cache the
    evaluated directory back onto the entry; the extractor always sets
    it to ``None``.
    """

    resolved_path: str | None
    raw: str
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


@dataclass(frozen=True, slots=True)
class WorkspaceDiagnosticContext:
    """Immutable cross-file context for diagnostics.

    Built once per diagnostic cycle from the workspace index on the
    event-loop thread, then passed to diagnostic functions running in
    background threads.  Frozen to guarantee thread safety.
    """

    workspace_proc_names: frozenset[str] = field(default_factory=frozenset)
    workspace_package_names: frozenset[str] = field(default_factory=frozenset)
    # Per-URI package names (for cross-file source graph resolution).
    package_names_by_uri: dict[str, frozenset[str]] = field(default_factory=dict)
    # Source dependency graph: uri -> set of URIs that file sources.
    source_graph: dict[str, frozenset[str]] = field(default_factory=dict)
    # Per-URI alias tail names (from ``interp alias`` definitions).
    alias_names_by_uri: dict[str, frozenset[str]] = field(default_factory=dict)


@dataclass(frozen=True, slots=True)
class UnknownProcInfo:
    """Analysis result from a user-defined ``unknown`` proc.

    When the analyser encounters ``proc unknown {cmd args} { ... }``, it
    inspects the body to determine which commands the handler can resolve.
    This information gates the W123 (unresolved command) diagnostic so
    that commands handled by ``unknown`` are not false-positived.
    """

    dispatch_targets: frozenset[str] = frozenset()
    """Command names explicitly dispatched (e.g. switch arm labels)."""
    chains_original: bool = False
    """Calls a renamed original ``unknown`` (e.g. ``_original_unknown``)."""
    empty_stub: bool = False
    """Body is empty — nothing resolves at all."""
    case_insensitive: bool = False
    """Normalises case before dispatch (all known commands are valid)."""
    has_pattern_dispatch: bool = False
    """Uses glob or regexp switch dispatch — opaque match semantics."""
    has_exec: bool = False
    """Calls ``exec`` — opaque external dispatch."""
    has_auto_load: bool = False
    """Calls ``auto_load`` — dynamic package loading."""


# Analysis result

# Sentinel values for ``AnalysisResult.suppressed_lines``.
# ``_NOQA_ALL`` is stored as the code-set value meaning "suppress every code
# on this line".  ``_FILE_SUPPRESS_KEY`` is the dict key used for codes that
# apply to the entire file (from a top-of-file ``# tcl-lsp: disable=`` directive).
# Negative keys can never collide with a real source line number.
_NOQA_ALL: frozenset[str] = frozenset({"*"})
_FILE_SUPPRESS_KEY: int = -1


@dataclass
class AnalysisResult:
    """Complete analysis result for a single document."""

    global_scope: Scope = field(default_factory=lambda: Scope(kind="global", name="::"))
    all_procs: dict[str, ProcDef] = field(default_factory=dict)
    all_classes: dict[str, ClassDef] = field(default_factory=dict)
    all_variables: dict[str, VarDef] = field(default_factory=dict)
    diagnostics: list[Diagnostic] = field(default_factory=list)
    # Suppressed diagnostic codes, keyed by line number.  Real line numbers
    # (>= 0) hold inline ``# <noqa>`` / ``# <noqa>: CODE`` codes; the sentinel
    # key ``-1`` holds codes from a top-of-file ``# tcl-lsp: disable=CODE``
    # directive and applies file-wide.  See
    # ``docs/kcs/kcs-howto-suppress-diagnostics.md``.
    suppressed_lines: dict[int, frozenset[str]] = field(default_factory=dict)
    regex_patterns: list[RegexPattern] = field(default_factory=list)
    command_invocations: list[CommandInvocation] = field(default_factory=list)
    method_invocations: list[MethodInvocation] = field(default_factory=list)
    package_requires: list[PackageRequire] = field(default_factory=list)
    package_provides: list[PackageProvide] = field(default_factory=list)
    has_dynamic_providers: bool = False  # True if load/auto_path detected
    source_targets: list[SourceTarget] = field(default_factory=list)
    namespace_imports: list[NamespaceImport] = field(default_factory=list)
    auto_path_entries: list[AutoPathEntry] = field(default_factory=list)
    stub_commands: list[StubCommandDef] = field(default_factory=list)
    stub_expr_defs: list[StubExprDef] = field(default_factory=list)
    # Command aliases: maps qualified alias_name (e.g. ``::=``) to
    # (target_cmd, prepended_args).  Populated from
    # ``interp alias {} name {} target ?arg ...?`` statements.
    command_aliases: dict[str, tuple[str, tuple[str, ...]]] = field(default_factory=dict)
    unknown_proc_info: UnknownProcInfo | None = None
    # Cached set of (line, character) for regex pattern positions,
    # built lazily by ``regex_position_set`` to avoid rebuilding on
    # every semantic token request.
    _regex_position_set: frozenset[tuple[int, int]] | None = field(
        default=None, repr=False, compare=False
    )
    # Bare-name index for O(1) proc lookup fallback, built eagerly
    # by ``_ensure_bare_name_index`` to avoid thread-unsafe lazy init.
    _bare_name_index: dict[str, ProcDef] | None = field(default=None, repr=False, compare=False)

    @property
    def regex_position_set(self) -> frozenset[tuple[int, int]]:
        """Return a frozenset of ``(line, character)`` for all regex patterns.

        Cached after first computation so repeated semantic token
        requests reuse the same set.
        """
        if self._regex_position_set is None:
            self._regex_position_set = frozenset(
                (rp.range.start.line, rp.range.start.character) for rp in self.regex_patterns
            )
        return self._regex_position_set

    def copy_for_snapshot(self) -> AnalysisResult:
        """Create an independent copy suitable for analyser snapshots.

        Much faster than ``copy.deepcopy`` because it shares immutable
        objects by reference (all ``frozen=True, slots=True`` dataclasses)
        and only copies mutable containers and their mutable items.

        ``ProcDef`` copies share the (already-immutable) ``params`` list
        items and ``param_traits`` frozenset values.  ``VarDef`` copies
        share immutable ``Range`` objects but must copy the mutable
        ``references`` list.
        """
        # Copy each ClassDef once; reuse the copy for both ``all_classes`` and
        # any scope that references the same live class (identity preserved).
        new_classes = {k: v.copy() for k, v in self.all_classes.items()}
        class_map = {id(v): new_classes[k] for k, v in self.all_classes.items()}
        new_scope = self.global_scope._copy_tree(class_map=class_map)
        # Share params list via shallow copy (ParamDef is frozen).
        # Share param_traits dict values (frozenset — immutable).
        new_procs = {
            k: ProcDef(
                v.name,
                v.qualified_name,
                v.params[:],
                v.name_range,
                v.body_range,
                v.doc,
                v.param_traits.copy(),
                v.is_trace_callback,
            )
            for k, v in self.all_procs.items()
        }
        new_vars = {
            k: VarDef(
                v.name,
                v.definition_range,
                v.references[:],
                v.warn_if_unused,
                set(v.array_indices),
            )
            for k, v in self.all_variables.items()
        }
        return AnalysisResult(
            global_scope=new_scope,
            all_procs=new_procs,
            all_classes=new_classes,
            all_variables=new_vars,
            diagnostics=self.diagnostics[:],
            suppressed_lines=self.suppressed_lines.copy(),
            regex_patterns=self.regex_patterns[:],
            command_invocations=self.command_invocations[:],
            method_invocations=self.method_invocations[:],
            package_requires=self.package_requires[:],
            package_provides=self.package_provides[:],
            has_dynamic_providers=self.has_dynamic_providers,
            source_targets=self.source_targets[:],
            namespace_imports=self.namespace_imports[:],
            auto_path_entries=self.auto_path_entries[:],
            stub_commands=self.stub_commands[:],
            stub_expr_defs=self.stub_expr_defs[:],
            command_aliases=self.command_aliases.copy(),
            unknown_proc_info=self.unknown_proc_info,
        )

    def for_index(self) -> AnalysisResult:
        """Return a lightweight copy retaining only fields cross-file readers touch.

        The workspace index and its callers read these fields on non-OPEN
        entries: ``all_procs`` / ``all_classes`` (symbol index),
        ``command_invocations`` (workspace usage counts),
        ``method_invocations`` (cross-file method reference counts),
        ``package_requires`` (workspace Tcl-version upgrade and
        ``active_package_names``), ``command_aliases`` (workspace
        diagnostics context), and ``source_targets`` (rename of a
        sourced file). Every other field is only consumed on the
        currently-open document, including ``global_scope``: its lone
        cross-file reader is ``WorkspaceIndex.find_var_in_scope``, which
        nothing outside the index itself calls, so retaining the scope
        tree would just pin child scopes and variable references for
        thousands of background-indexed files without observable
        benefit.
        """
        return AnalysisResult(
            all_procs=self.all_procs,
            all_classes=self.all_classes,
            command_invocations=self.command_invocations,
            method_invocations=self.method_invocations,
            package_requires=self.package_requires,
            command_aliases=self.command_aliases,
            source_targets=self.source_targets,
            namespace_imports=self.namespace_imports,
            auto_path_entries=self.auto_path_entries,
        )

    def _ensure_bare_name_index(self) -> dict[str, ProcDef]:
        """Build or return the bare-name → ProcDef index.

        Called once; subsequent accesses use the cached dict.
        Thread-safe because the index is a dict (atomic pointer swap
        under the GIL) and the result is idempotent.
        """
        idx = self._bare_name_index
        if idx is None:
            idx = {}
            for pd in self.all_procs.values():
                idx.setdefault(pd.name, pd)
            self._bare_name_index = idx
        return idx

    def find_proc(self, name: str) -> ProcDef | None:
        """Look up a proc by name, trying qualified and bare forms.

        Uses a bare-name index for O(1) amortised lookup instead of
        O(P) linear scan.
        """
        result = self.all_procs.get(f"::{name}") or self.all_procs.get(name)
        if result is not None:
            return result
        return self._ensure_bare_name_index().get(name)

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
