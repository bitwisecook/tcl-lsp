"""Cross-file symbol index.

Aggregates proc definitions and variable definitions across all indexed
documents -- both currently-open files and background-scanned workspace/library
files -- enabling cross-file go-to-definition, find-references, and completion.
"""

from __future__ import annotations

import threading
from dataclasses import dataclass
from enum import Enum, auto

from core.analysis.semantic_model import AnalysisResult, ProcDef, Range, Scope, VarDef


class EntrySource(Enum):
    """How a file ended up in the index."""

    OPEN = auto()  # File is open in the editor
    BACKGROUND = auto()  # File scanned from workspace on disk
    PACKAGE = auto()  # File loaded via package require resolution


@dataclass
class IndexEntry:
    """A symbol entry in the workspace index."""

    uri: str
    proc: ProcDef | None = None
    var: VarDef | None = None
    scope: Scope | None = None
    source_kind: EntrySource = EntrySource.OPEN


@dataclass(frozen=True, slots=True)
class RuleInitVarDef:
    """A variable or array set in a RULE_INIT block (cross-file metadata)."""

    name: str  # e.g. "::my_var"
    source_uri: str
    priority: int  # RULE_INIT priority (default 500)
    definition_range: Range
    is_array: bool = False


class WorkspaceIndex:
    """Cross-file symbol index built from per-document AnalysisResults."""

    def __init__(self) -> None:
        self._lock = threading.Lock()
        # qualified_name -> list of entries (could be redefined across files)
        self._procs: dict[str, list[IndexEntry]] = {}
        self._per_uri: dict[str, AnalysisResult] = {}
        self._source_kinds: dict[str, EntrySource] = {}
        # Reverse indexes for O(1) tail-name lookup
        self._tail_index: dict[str, set[str]] = {}  # simple_name -> {qualified_names}
        self._irules_tail_index: dict[str, set[str]] = {}
        # Reverse index for targeted removal: uri -> {qnames contributed}
        self._uri_to_qnames: dict[str, set[str]] = {}
        # Cached usage counts (invalidated on mutation)
        self._command_usage_cache: dict[str, int] | None = None
        self._proc_usage_cache: dict[str, int] | None = None
        # iRules: procs from any iRules file are globally available
        self._irules_global_procs: dict[str, list[IndexEntry]] = {}
        # iRules: cross-file RULE_INIT variables, keyed by normalised var name
        self._irules_rule_init_vars: dict[str, list[RuleInitVarDef]] = {}

    def update(
        self,
        uri: str,
        result: AnalysisResult,
        source_kind: EntrySource = EntrySource.OPEN,
    ) -> None:
        """Update the index for a given document."""
        with self._lock:
            self._remove_unlocked(uri)
            self._per_uri[uri] = result
            self._source_kinds[uri] = source_kind

            qnames_for_uri: set[str] = set()
            for qname, proc_def in result.all_procs.items():
                entry = IndexEntry(uri=uri, proc=proc_def, source_kind=source_kind)
                self._procs.setdefault(qname, []).append(entry)
                # Maintain tail index
                tail = qname.rsplit("::", 1)[-1]
                self._tail_index.setdefault(tail, set()).add(qname)
                qnames_for_uri.add(qname)
            self._uri_to_qnames[uri] = qnames_for_uri

    def remove(self, uri: str) -> None:
        """Remove all entries for a URI."""
        with self._lock:
            self._remove_unlocked(uri)

    def _remove_unlocked(self, uri: str) -> None:
        """Remove all entries for a URI (caller must hold lock)."""
        old = self._per_uri.pop(uri, None)
        self._source_kinds.pop(uri, None)
        self._command_usage_cache = None
        self._proc_usage_cache = None
        if old is None:
            self._uri_to_qnames.pop(uri, None)
            return
        # Only iterate qnames this URI contributed
        for qname in self._uri_to_qnames.pop(uri, ()):
            if qname in self._procs:
                self._procs[qname] = [e for e in self._procs[qname] if e.uri != uri]
                if not self._procs[qname]:
                    del self._procs[qname]
                    # Clean tail index
                    tail = qname.rsplit("::", 1)[-1]
                    if tail in self._tail_index:
                        self._tail_index[tail].discard(qname)
                        if not self._tail_index[tail]:
                            del self._tail_index[tail]
        # RULE_INIT vars: small enough that linear scan is fine
        for vname in list(self._irules_rule_init_vars):
            self._irules_rule_init_vars[vname] = [
                e for e in self._irules_rule_init_vars[vname] if e.source_uri != uri
            ]
            if not self._irules_rule_init_vars[vname]:
                del self._irules_rule_init_vars[vname]

    def update_irules_globals(self, uri: str, procs: dict[str, ProcDef]) -> None:
        """Register all procs from an iRules file as globally available."""
        with self._lock:
            # Remove old entries for this URI
            for qname in list(self._irules_global_procs):
                self._irules_global_procs[qname] = [
                    e for e in self._irules_global_procs[qname] if e.uri != uri
                ]
                if not self._irules_global_procs[qname]:
                    del self._irules_global_procs[qname]
                    tail = qname.rsplit("::", 1)[-1]
                    if tail in self._irules_tail_index:
                        self._irules_tail_index[tail].discard(qname)
                        if not self._irules_tail_index[tail]:
                            del self._irules_tail_index[tail]
            # Add new entries
            for qname, proc_def in procs.items():
                entry = IndexEntry(
                    uri=uri,
                    proc=proc_def,
                    source_kind=EntrySource.BACKGROUND,
                )
                self._irules_global_procs.setdefault(qname, []).append(entry)
                tail = qname.rsplit("::", 1)[-1]
                self._irules_tail_index.setdefault(tail, set()).add(qname)

    def update_rule_init_vars(
        self,
        uri: str,
        exports: list,
    ) -> None:
        """Register variables exported from RULE_INIT blocks."""
        with self._lock:
            # Remove old entries for this URI
            for vname in list(self._irules_rule_init_vars):
                self._irules_rule_init_vars[vname] = [
                    e for e in self._irules_rule_init_vars[vname] if e.source_uri != uri
                ]
                if not self._irules_rule_init_vars[vname]:
                    del self._irules_rule_init_vars[vname]
            # Add new entries
            for exp in exports:
                entry = RuleInitVarDef(
                    name=exp.name,
                    source_uri=uri,
                    priority=exp.priority,
                    definition_range=exp.range,
                    is_array=exp.is_array,
                )
                self._irules_rule_init_vars.setdefault(exp.name, []).append(entry)

    def find_rule_init_var(self, name: str) -> list[RuleInitVarDef]:
        """Find RULE_INIT variable definitions by name."""
        with self._lock:
            return list(self._irules_rule_init_vars.get(name, []))

    def all_rule_init_var_names(self) -> list[str]:
        """Return all known RULE_INIT variable names."""
        with self._lock:
            return list(self._irules_rule_init_vars.keys())

    def find_proc(self, name: str) -> list[IndexEntry]:
        """Find proc definitions by qualified or simple name."""
        with self._lock:
            results = self._find_in_dict(name, self._procs, self._tail_index)
            results.extend(
                self._find_in_dict(name, self._irules_global_procs, self._irules_tail_index)
            )
            # Deduplicate by (uri, qualified_name)
            seen: set[tuple[str, str]] = set()
            deduped: list[IndexEntry] = []
            for entry in results:
                key = (entry.uri, entry.proc.qualified_name if entry.proc else "")
                if key not in seen:
                    seen.add(key)
                    deduped.append(entry)
            return deduped

    @staticmethod
    def _find_in_dict(
        name: str,
        procs: dict[str, list[IndexEntry]],
        tail_index: dict[str, set[str]],
    ) -> list[IndexEntry]:
        """Look up a proc name in a proc dict (exact, qualified, or tail)."""
        if name in procs:
            return list(procs[name])
        qualified = f"::{name}"
        if qualified in procs:
            return list(procs[qualified])
        # O(1) tail-name lookup instead of O(n) scan
        results: list[IndexEntry] = []
        for qname in tail_index.get(name, ()):
            if qname in procs:
                results.extend(procs[qname])
        return results

    def all_proc_names(self) -> list[str]:
        """Return all known proc qualified names."""
        with self._lock:
            names = set(self._procs.keys())
            names.update(self._irules_global_procs.keys())
            return list(names)

    def command_usage_counts(self) -> dict[str, int]:
        """Return workspace-wide invocation counts keyed by command name."""
        with self._lock:
            if self._command_usage_cache is None:
                counts: dict[str, int] = {}
                for result in self._per_uri.values():
                    for invocation in result.command_invocations:
                        counts[invocation.name] = counts.get(invocation.name, 0) + 1
                self._command_usage_cache = counts
            return dict(self._command_usage_cache)

    def proc_usage_counts(self) -> dict[str, int]:
        """Return workspace-wide invocation counts keyed by resolved proc qname."""
        with self._lock:
            if self._proc_usage_cache is None:
                counts: dict[str, int] = {}
                for result in self._per_uri.values():
                    for invocation in result.command_invocations:
                        qname = invocation.resolved_qualified_name
                        if not qname:
                            continue
                        counts[qname] = counts.get(qname, 0) + 1
                self._proc_usage_cache = counts
            return dict(self._proc_usage_cache)

    def all_uris(self) -> list[str]:
        """Return all indexed URIs."""
        with self._lock:
            return list(self._per_uri.keys())

    def is_background(self, uri: str) -> bool:
        """Check if a URI is a background (non-open) entry."""
        with self._lock:
            return self._source_kinds.get(uri) in (
                EntrySource.BACKGROUND,
                EntrySource.PACKAGE,
            )

    def remove_background_entries(self) -> None:
        """Remove all non-OPEN entries (for rescanning)."""
        with self._lock:
            bg_uris = [u for u, k in self._source_kinds.items() if k != EntrySource.OPEN]
            for uri in bg_uris:
                self._remove_unlocked(uri)
            self._irules_global_procs.clear()
            self._irules_tail_index.clear()
            self._irules_rule_init_vars.clear()

    def find_var_in_scope(
        self,
        name: str,
        uri: str,
        scope: Scope | None = None,
    ) -> VarDef | None:
        """Find a variable definition, walking up the scope chain."""
        if scope is not None:
            if name in scope.variables:
                return scope.variables[name]
            if scope.parent is not None:
                return self.find_var_in_scope(name, uri, scope.parent)
        # Check global scope of the document
        with self._lock:
            result = self._per_uri.get(uri)
        if result and name in result.global_scope.variables:
            return result.global_scope.variables[name]
        return None

    def get_analysis(self, uri: str) -> AnalysisResult | None:
        """Get the analysis result for a URI."""
        with self._lock:
            return self._per_uri.get(uri)
