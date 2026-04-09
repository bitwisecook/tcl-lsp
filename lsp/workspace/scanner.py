"""Background workspace scanner.

Scans workspace directories and configured library paths for Tcl files,
runs lightweight analysis (proc signature extraction only via ``analyse()``),
and populates the WorkspaceIndex with background entries.
"""

from __future__ import annotations

import logging
import os
import threading
import time
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path
from urllib.parse import quote, unquote, urlparse

from core.analysis.analyser import analyse
from core.analysis.semantic_model import AnalysisResult, ProcDef
from core.bigip.apl_model import AplModel, resolve_apl_includes
from core.bigip.model import BigipConfig
from core.bigip.parser import parse_bigip_conf
from core.compiler.irules_flow import RuleInitExport

log = logging.getLogger(__name__)

TCL_EXTENSIONS = frozenset(
    {
        ".tcl",
        ".tk",
        ".itcl",
        ".tm",
        ".irul",
        ".irule",
        ".iapp",
        ".iappimpl",
        ".impl",
        ".exp",
        ".apl",
    }
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

# APL presentation file names (matched by basename, no extension).
_APL_NAMES = frozenset({"presentation"})


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
        self._apl_models: dict[str, AplModel] = {}  # uri -> AplModel
        self._auto_index_entries: dict[str, str] = {}  # proc_name -> abs path

    def configure(
        self,
        workspace_roots: list[str] | None = None,
        library_paths: list[str] | None = None,
    ) -> None:
        if workspace_roots is not None:
            self._workspace_roots = list(workspace_roots)
            # New roots → invalidate all cached results.
            self._cached.clear()
        if library_paths is not None:
            self._library_paths = list(library_paths)

    def collect_files(self) -> list[tuple[str, str]]:
        """Discover all Tcl and BIG-IP config files without analysing them.

        Returns a list of ``(full_path, ext)`` pairs for Tcl files.
        BIG-IP config files are parsed eagerly (they're lightweight).
        tclIndex files are parsed to discover additional source files.
        """
        self._bigip_configs.clear()
        self._auto_index_entries.clear()
        all_dirs = self._workspace_roots + self._library_paths
        result: list[tuple[str, str]] = []
        seen_paths: set[str] = set()

        for dir_path in all_dirs:
            expanded = os.path.expanduser(dir_path)
            if not os.path.isdir(expanded):
                log.warning("Scanner: directory not found: %s", dir_path)
                continue
            for root, _dirs, files in os.walk(expanded):
                # Parse tclIndex files to discover auto-loaded proc mappings.
                files_lower = {f.lower(): f for f in files}
                if "tclindex" in files_lower:
                    tcl_index_path = os.path.join(root, files_lower["tclindex"])
                    self._parse_auto_index(tcl_index_path, result, seen_paths)

                for fname in files:
                    fname_lower = fname.lower()
                    # Skip tclIndex itself — it is metadata, not Tcl code.
                    if fname_lower == "tclindex":
                        continue
                    # Check for BIG-IP configuration files
                    if fname_lower in _BIGIP_CONF_NAMES:
                        full_path = os.path.join(root, fname)
                        self._parse_bigip_file(full_path)
                        continue
                    # APL presentation files (extensionless)
                    if fname_lower in _APL_NAMES:
                        full_path = os.path.join(root, fname)
                        if full_path not in seen_paths:
                            seen_paths.add(full_path)
                            result.append((full_path, ".apl"))
                        continue
                    ext = os.path.splitext(fname)[1].lower()
                    if ext not in TCL_EXTENSIONS:
                        continue
                    full_path = os.path.join(root, fname)
                    if full_path not in seen_paths:
                        seen_paths.add(full_path)
                        result.append((full_path, ext))
        return result

    def _parse_auto_index(
        self,
        tcl_index_path: str,
        result: list[tuple[str, str]],
        seen_paths: set[str],
    ) -> None:
        """Parse a tclIndex file and register its proc->file mappings."""
        from core.packages.auto_index import parse_tcl_index

        entries = parse_tcl_index(tcl_index_path)
        for entry in entries:
            self._auto_index_entries[entry.proc_name] = entry.source_file
            # Ensure referenced files are included in the scan list.
            if entry.source_file not in seen_paths:
                seen_paths.add(entry.source_file)
                ext = os.path.splitext(entry.source_file)[1].lower()
                if not ext:
                    ext = ".tcl"
                result.append((entry.source_file, ext))

    def analyse_one(self, full_path: str, ext: str) -> ScanResult | None:
        """Analyse a single file and cache the result."""
        return self._analyse_file(full_path, ext)

    # Maximum time (seconds) to spend analysing a single file during
    # the background scan.  Files that exceed this are skipped.
    PER_FILE_TIMEOUT: float = 10.0

    def scan_all(
        self,
        skip_uris: frozenset[str] = frozenset(),
        progress_cb: Callable[[int, int, str], None] | None = None,
    ) -> dict[str, ScanResult]:
        """Scan all configured directories.  Returns all results.

        Each file is analysed with a timeout so that a single
        pathological file cannot stall the entire scan.

        *skip_uris* is an optional set of URIs to skip (e.g. files
        already open and analysed via ``didOpen``).

        *progress_cb* is an optional callback invoked as
        ``progress_cb(index, total, file_path)`` after each file is
        processed.  Implementations must be fast and non-blocking —
        typically they marshal a notification onto the event loop.
        """
        t_start = time.perf_counter()
        files = self.collect_files()
        t_collect = time.perf_counter()
        log.info(
            "[timing] collect_files %.0fms (%d files found)",
            (t_collect - t_start) * 1000,
            len(files),
        )

        total_files = len(files)
        skipped_open = 0
        skipped_cached = 0
        discovered_uris: set[str] = set()
        for idx, (full_path, ext) in enumerate(files, start=1):
            uri = path_to_uri(full_path)
            discovered_uris.add(uri)
            if progress_cb is not None:
                try:
                    progress_cb(idx, total_files, full_path)
                except Exception:  # pragma: no cover - defensive
                    log.debug("progress_cb raised", exc_info=True)
            if skip_uris and uri in skip_uris:
                skipped_open += 1
                continue
            if uri in self._cached:
                skipped_cached += 1
                continue
            self._analyse_file_with_timeout(full_path, ext)
            # Yield the GIL between files so the asyncio event
            # loop's stdin reader thread can make progress.
            time.sleep(0)
        if skipped_open or skipped_cached:
            log.info(
                "Scanner: skipped %d open + %d cached files",
                skipped_open,
                skipped_cached,
            )

        # Prune stale entries for files that no longer exist.
        stale = set(self._cached) - discovered_uris
        for uri in stale:
            del self._cached[uri]

        elapsed_ms = (time.perf_counter() - t_start) * 1000
        log.info(
            "[timing] scan_all %.0fms total (%d files, %d bigip configs)",
            elapsed_ms,
            len(self._cached),
            len(self._bigip_configs),
        )
        return dict(self._cached)

    def _analyse_file_with_timeout(
        self,
        full_path: str,
        ext: str,
    ) -> ScanResult | None:
        """Run _analyse_file with a per-file timeout.

        Uses a daemon thread so that a hung analysis does not block
        the scan indefinitely.  The analysis result is written to
        ``_cached`` only after the thread completes within the timeout,
        so a timed-out thread cannot mutate shared state later.
        """
        t0 = time.perf_counter()
        result_box: list[ScanResult | None] = [None]

        def _work() -> None:
            result_box[0] = self._run_analysis(full_path, ext)

        t = threading.Thread(target=_work, daemon=True)
        t.start()
        t.join(timeout=self.PER_FILE_TIMEOUT)
        if t.is_alive():
            log.warning(
                "Scanner: analysis timed out after %.0fs for %s",
                self.PER_FILE_TIMEOUT,
                full_path,
            )
            return None
        elapsed_ms = (time.perf_counter() - t0) * 1000
        if elapsed_ms > 500:
            log.info("[timing] scanner file %.0fms %s", elapsed_ms, full_path)
        # Only cache after a successful, timely completion.
        result = result_box[0]
        if result is not None:
            self._cached[result.uri] = result
        return result

    def rescan_file(self, file_path: str) -> ScanResult | None:
        """Re-scan a single file (e.g. after a filesystem change)."""
        ext = os.path.splitext(file_path)[1].lower()
        return self._analyse_file(file_path, ext)

    @property
    def auto_index_entries(self) -> dict[str, str]:
        """Return proc_name -> source_file mappings from tclIndex files."""
        return dict(self._auto_index_entries)

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

    # APL model caching

    @property
    def apl_models(self) -> dict[str, AplModel]:
        """Return uri -> AplModel for all cached APL presentation files."""
        return dict(self._apl_models)

    def parse_apl_source(
        self, uri: str, source: str, base_dir: str | None = None
    ) -> AplModel | None:
        """Parse an APL presentation source and cache the result."""
        try:
            model = resolve_apl_includes(source, base_dir)
            self._apl_models[uri] = model
            return model
        except Exception:
            log.debug("Scanner: failed to parse APL %s", uri, exc_info=True)
            return None

    def remove_apl_model(self, uri: str) -> None:
        self._apl_models.pop(uri, None)

    def find_sibling_apl(self, uri: str) -> AplModel | None:
        """Find a cached APL model from the same directory as *uri*."""
        # Extract directory from the URI
        if "/" in uri:
            dir_part = uri.rsplit("/", 1)[0]
        else:
            return None
        for apl_uri, model in self._apl_models.items():
            if "/" in apl_uri and apl_uri.rsplit("/", 1)[0] == dir_part:
                return model
        return None

    def find_sibling_impl_source(self, uri: str) -> str | None:
        """Find a cached iApp implementation source from the same directory.

        Returns the URI of the sibling implementation file, or None.
        """
        if "/" in uri:
            dir_part = uri.rsplit("/", 1)[0]
        else:
            return None
        for cached_uri, sr in self._cached.items():
            if "/" not in cached_uri:
                continue
            if cached_uri.rsplit("/", 1)[0] != dir_part:
                continue
            if sr.dialect_hint == "f5-iapps":
                return cached_uri
        return None

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

    def _run_analysis(self, full_path: str, ext: str) -> ScanResult | None:
        """Run analysis for a single file without caching the result."""
        uri = path_to_uri(full_path)
        try:
            source = Path(full_path).read_text(encoding="utf-8", errors="replace")
            result = analyse(source)
            dialect = _dialect_from_ext(ext)
            rule_init_exports: list[RuleInitExport] = []
            if dialect == "f5-irules":
                from core.compiler.irules_flow import extract_rule_init_vars

                rule_init_exports = extract_rule_init_vars(source)
            return ScanResult(
                uri=uri,
                file_path=full_path,
                analysis=result,
                dialect_hint=dialect,
                rule_init_exports=rule_init_exports,
            )
        except Exception:
            log.debug("Scanner: failed to analyse %s", full_path, exc_info=True)
            return None

    def _analyse_file(self, full_path: str, ext: str) -> ScanResult | None:
        """Analyse a single file and cache the result."""
        scan_result = self._run_analysis(full_path, ext)
        if scan_result is not None:
            self._cached[scan_result.uri] = scan_result
        return scan_result


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
    if ext == ".apl":
        return "f5-iapps"
    if ext == ".exp":
        return "expect"
    return None
