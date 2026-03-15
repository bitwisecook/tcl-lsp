"""Background workspace scanner.

Scans workspace directories and configured library paths for Tcl files,
runs lightweight analysis (proc signature extraction only via ``analyse()``),
and populates the WorkspaceIndex with background entries.
"""

from __future__ import annotations

import logging
import os
from dataclasses import dataclass, field
from pathlib import Path
from urllib.parse import quote, unquote, urlparse

from core.analysis.analyser import analyse
from core.analysis.semantic_model import AnalysisResult, ProcDef
from core.bigip.model import BigipConfig
from core.bigip.parser import parse_bigip_conf
from core.compiler.irules_flow import RuleInitExport

log = logging.getLogger(__name__)

TCL_EXTENSIONS = frozenset(
    {".tcl", ".tk", ".itcl", ".tm", ".irul", ".irule", ".iapp", ".iappimpl", ".impl", ".exp"}
)

_IRULES_EXTENSIONS = frozenset({".irul", ".irule"})

# BIG-IP configuration file names (matched by basename, not extension).
_BIGIP_CONF_NAMES = frozenset(
    {
        "bigip.conf",
        "bigip_base.conf",
        "bigip_gtm.conf",
        "bigip_script.conf",
        "bigip_user.conf",
    }
)


@dataclass
class ScanResult:
    """Lightweight analysis result for a single scanned file."""

    uri: str
    file_path: str
    analysis: AnalysisResult
    dialect_hint: str | None = None  # inferred from extension
    rule_init_exports: list[RuleInitExport] = field(default_factory=list)


class BackgroundScanner:
    """Scans directories for Tcl files and caches lightweight analysis results."""

    def __init__(self) -> None:
        self._cached: dict[str, ScanResult] = {}  # uri -> ScanResult
        self._library_paths: list[str] = []
        self._workspace_roots: list[str] = []
        self._bigip_configs: dict[str, BigipConfig] = {}  # uri -> BigipConfig

    def configure(
        self,
        workspace_roots: list[str] | None = None,
        library_paths: list[str] | None = None,
    ) -> None:
        if workspace_roots is not None:
            self._workspace_roots = list(workspace_roots)
        if library_paths is not None:
            self._library_paths = list(library_paths)

    def scan_all(self) -> dict[str, ScanResult]:
        """Scan all configured directories.  Returns all results."""
        self._cached.clear()
        self._bigip_configs.clear()
        all_dirs = self._workspace_roots + self._library_paths

        for dir_path in all_dirs:
            expanded = os.path.expanduser(dir_path)
            if not os.path.isdir(expanded):
                log.warning("Scanner: directory not found: %s", dir_path)
                continue
            for root, _dirs, files in os.walk(expanded):
                for fname in files:
                    fname_lower = fname.lower()
                    # Check for BIG-IP configuration files
                    if fname_lower in _BIGIP_CONF_NAMES:
                        full_path = os.path.join(root, fname)
                        self._parse_bigip_file(full_path)
                        continue
                    ext = os.path.splitext(fname)[1].lower()
                    if ext not in TCL_EXTENSIONS:
                        continue
                    full_path = os.path.join(root, fname)
                    self._analyse_file(full_path, ext)

        log.info(
            "Background scan complete: indexed %d files, %d bigip configs",
            len(self._cached),
            len(self._bigip_configs),
        )
        return dict(self._cached)

    def rescan_file(self, file_path: str) -> ScanResult | None:
        """Re-scan a single file (e.g. after a filesystem change)."""
        ext = os.path.splitext(file_path)[1].lower()
        return self._analyse_file(file_path, ext)

    def has_cached(self, uri: str) -> bool:
        return uri in self._cached

    def get_cached(self, uri: str) -> AnalysisResult | None:
        sr = self._cached.get(uri)
        return sr.analysis if sr else None

    def remove_file(self, uri: str) -> None:
        self._cached.pop(uri, None)

    @property
    def irules_procs(self) -> dict[str, dict[str, ProcDef]]:
        """Return uri -> {qname: ProcDef} for all cached iRules files."""
        result: dict[str, dict[str, ProcDef]] = {}
        for uri, sr in self._cached.items():
            if sr.dialect_hint == "f5-irules" and sr.analysis.all_procs:
                result[uri] = dict(sr.analysis.all_procs)
        return result

    @property
    def irules_rule_init_vars(self) -> dict[str, list[RuleInitExport]]:
        """Return uri -> [RuleInitExport] for all cached iRules files."""
        result: dict[str, list[RuleInitExport]] = {}
        for uri, sr in self._cached.items():
            if sr.rule_init_exports:
                result[uri] = list(sr.rule_init_exports)
        return result

    @property
    def bigip_configs(self) -> dict[str, BigipConfig]:
        """Return uri -> BigipConfig for all cached BIG-IP configuration files."""
        return dict(self._bigip_configs)

    @property
    def merged_bigip_config(self) -> BigipConfig | None:
        """Return a merged BigipConfig from all scanned conf files, or None."""
        if not self._bigip_configs:
            return None
        merged = BigipConfig()
        for cfg in self._bigip_configs.values():
            merged.data_groups.update(cfg.data_groups)
            merged.pools.update(cfg.pools)
            merged.virtual_servers.update(cfg.virtual_servers)
            merged.nodes.update(cfg.nodes)
            merged.profiles.update(cfg.profiles)
            merged.monitors.update(cfg.monitors)
            merged.snat_pools.update(cfg.snat_pools)
            merged.persistence.update(cfg.persistence)
            merged.rules.update(cfg.rules)
            merged.generic_objects.update(cfg.generic_objects)
        return merged

    def parse_bigip_source(self, uri: str, source: str) -> BigipConfig | None:
        """Parse a BIG-IP config from source text and cache the result."""
        try:
            config = parse_bigip_conf(source)
            self._bigip_configs[uri] = config
            return config
        except Exception:
            log.debug("Scanner: failed to parse bigip config %s", uri, exc_info=True)
            return None

    def remove_bigip_config(self, uri: str) -> None:
        self._bigip_configs.pop(uri, None)

    def _parse_bigip_file(self, full_path: str) -> BigipConfig | None:
        """Parse a BIG-IP configuration file and cache the result."""
        uri = path_to_uri(full_path)
        try:
            source = Path(full_path).read_text(encoding="utf-8", errors="replace")
            config = parse_bigip_conf(source)
            self._bigip_configs[uri] = config
            log.debug("Scanner: parsed bigip config %s", full_path)
            return config
        except Exception:
            log.debug("Scanner: failed to parse bigip config %s", full_path, exc_info=True)
            return None

    def _analyse_file(self, full_path: str, ext: str) -> ScanResult | None:
        uri = path_to_uri(full_path)
        try:
            source = Path(full_path).read_text(encoding="utf-8", errors="replace")
            result = analyse(source)
            dialect = _dialect_from_ext(ext)
            rule_init_exports: list[RuleInitExport] = []
            if dialect == "f5-irules":
                from core.compiler.irules_flow import extract_rule_init_vars

                rule_init_exports = extract_rule_init_vars(source)
            scan_result = ScanResult(
                uri=uri,
                file_path=full_path,
                analysis=result,
                dialect_hint=dialect,
                rule_init_exports=rule_init_exports,
            )
            self._cached[uri] = scan_result
            return scan_result
        except Exception:
            log.debug("Scanner: failed to analyse %s", full_path, exc_info=True)
            return None


# URI / path utilities


def path_to_uri(file_path: str) -> str:
    """Convert a filesystem path to a file:// URI."""
    abs_path = os.path.abspath(file_path)
    return "file://" + quote(abs_path, safe="/:")


def uri_to_path(uri: str) -> str | None:
    """Convert a file:// URI to a filesystem path."""
    parsed = urlparse(uri)
    if parsed.scheme != "file":
        return None
    return unquote(parsed.path)


def _dialect_from_ext(ext: str) -> str | None:
    if ext in _IRULES_EXTENSIONS:
        return "f5-irules"
    if ext in (".iapp", ".iappimpl", ".impl"):
        return "f5-iapps"
    if ext == ".exp":
        return "expect"
    return None
