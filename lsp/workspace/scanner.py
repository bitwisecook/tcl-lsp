"""Background workspace scanner.

Scans workspace directories and configured library paths for Tcl files,
runs the lightweight ``extract_signatures()`` pass (signature-only:
procs, classes, package requires, source targets, command aliases, and a
name-only invocation list), and populates the WorkspaceIndex with
background entries.
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

from compiler.irules_flow import RuleInitExport
from core.analysis.semantic_model import AnalysisResult, ProcDef
from core.analysis.signature_scan import extract_signatures
from dialects.f5.bigip.apl_model import AplModel, resolve_apl_includes
from dialects.f5.bigip.model import BigipConfig
from dialects.f5.bigip.parser import parse_bigip_conf

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
# Canonical-name path — these always parse as BIG-IP no matter the
# directory or surrounding files.
_BIGIP_CONF_NAMES = frozenset(
    {
        "bigip.conf",
        "bigip_base.conf",
        "bigip_gtm.conf",
        "bigip_script.conf",
        "bigip_user.conf",
    }
)

# BIG-IP file extensions for discovery beyond the canonical basenames.
# ``.scf`` is the F5 single-config-file export format (always BIG-IP);
# ``.conf`` files are tried opportunistically — the parser is tolerant
# of non-BIG-IP content (returns an empty :class:`BigipConfig` when
# nothing recognisable is present), so the worst case is a wasted
# parse rather than a false-positive entry in the inventory.
_BIGIP_FILE_EXTENSIONS = frozenset({".scf", ".conf"})

# Skip files larger than this when discovering arbitrary ``.conf`` /
# ``.scf`` paths.  Real BIG-IP exports cap out at the low single-digit
# MB range; anything bigger is more likely to be a log, dump, or
# unrelated artefact than a config the LSP needs to index.
_BIGIP_MAX_FILE_BYTES = 32 * 1024 * 1024

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
        self._lock = threading.RLock()
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
        with self._lock:
            if workspace_roots is not None:
                self._workspace_roots = list(workspace_roots)
                # New roots → invalidate all cached results.
                self._cached.clear()
            if library_paths is not None:
                self._library_paths = list(library_paths)

    @property
    def workspace_roots(self) -> list[str]:
        """Return a snapshot of the currently configured workspace roots."""
        with self._lock:
            return list(self._workspace_roots)

    def collect_files(self) -> list[tuple[str, str]]:
        """Discover all Tcl and BIG-IP config files without analysing them.

        Returns a list of ``(full_path, ext)`` pairs for Tcl files.
        BIG-IP config files are parsed eagerly (they're lightweight).
        tclIndex files are parsed to discover additional source files.
        """
        with self._lock:
            all_dirs = self._workspace_roots + self._library_paths
        bigip_configs: dict[str, BigipConfig] = {}
        auto_index_entries: dict[str, str] = {}
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
                    self._parse_auto_index(
                        tcl_index_path,
                        result,
                        seen_paths,
                        auto_index_entries,
                    )

                for fname in files:
                    fname_lower = fname.lower()
                    # Skip tclIndex itself — it is metadata, not Tcl code.
                    if fname_lower == "tclindex":
                        continue
                    # BIG-IP config discovery — canonical basenames
                    # always; ``.scf`` always; ``.conf`` opportunistic
                    # (parser is tolerant of non-BIG-IP content, so a
                    # wasted parse is the worst case).  Size-capped so
                    # we don't slurp dump files / logs that happen to
                    # share an extension.
                    if fname_lower in _BIGIP_CONF_NAMES:
                        full_path = os.path.join(root, fname)
                        self._parse_bigip_file(full_path, bigip_configs)
                        continue
                    ext = os.path.splitext(fname_lower)[1]
                    if ext in _BIGIP_FILE_EXTENSIONS:
                        full_path = os.path.join(root, fname)
                        try:
                            if os.path.getsize(full_path) > _BIGIP_MAX_FILE_BYTES:
                                continue
                        except OSError:
                            continue
                        self._parse_bigip_file(full_path, bigip_configs)
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
        with self._lock:
            self._bigip_configs = bigip_configs
            self._auto_index_entries = auto_index_entries
        return result

    def _parse_auto_index(
        self,
        tcl_index_path: str,
        result: list[tuple[str, str]],
        seen_paths: set[str],
        auto_index_entries: dict[str, str],
    ) -> None:
        """Parse a tclIndex file and register its proc->file mappings."""
        from core.packages.auto_index import parse_tcl_index

        entries = parse_tcl_index(tcl_index_path)
        for entry in entries:
            auto_index_entries[entry.proc_name] = entry.source_file
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
            if skip_uris and uri in skip_uris:
                skipped_open += 1
            elif self.has_cached(uri):
                skipped_cached += 1
            else:
                self._analyse_file_with_timeout(full_path, ext)
                # Yield the GIL between files so the asyncio event
                # loop's stdin reader thread can make progress.
                time.sleep(0)
            # Progress is reported *after* each file has either been
            # analysed or skipped — never before — so the percentage
            # reflects work actually completed and never regresses.
            if progress_cb is not None:
                try:
                    progress_cb(idx, total_files, full_path)
                except Exception:  # pragma: no cover - defensive
                    log.debug("progress_cb raised", exc_info=True)
        if skipped_open or skipped_cached:
            log.info(
                "Scanner: skipped %d open + %d cached files",
                skipped_open,
                skipped_cached,
            )

        # Prune stale entries for files that no longer exist.
        with self._lock:
            stale = set(self._cached) - discovered_uris
            for uri in stale:
                del self._cached[uri]
            cached_count = len(self._cached)
            bigip_count = len(self._bigip_configs)
            results = dict(self._cached)

        elapsed_ms = (time.perf_counter() - t_start) * 1000
        log.info(
            "[timing] scan_all %.0fms total (%d files, %d bigip configs)",
            elapsed_ms,
            cached_count,
            bigip_count,
        )
        return results

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
            with self._lock:
                self._cached[result.uri] = result
        return result

    def rescan_file(self, file_path: str) -> ScanResult | None:
        """Re-scan a single file (e.g. after a filesystem change)."""
        ext = os.path.splitext(file_path)[1].lower()
        return self._analyse_file(file_path, ext)

    @property
    def auto_index_entries(self) -> dict[str, str]:
        """Return proc_name -> source_file mappings from tclIndex files."""
        with self._lock:
            return dict(self._auto_index_entries)

    def has_cached(self, uri: str) -> bool:
        with self._lock:
            return uri in self._cached

    def get_cached(self, uri: str) -> AnalysisResult | None:
        with self._lock:
            sr = self._cached.get(uri)
        return sr.analysis if sr else None

    def remove_file(self, uri: str) -> None:
        with self._lock:
            self._cached.pop(uri, None)

    @property
    def irules_procs(self) -> dict[str, dict[str, ProcDef]]:
        """Return uri -> {qname: ProcDef} for all cached iRules files."""
        result: dict[str, dict[str, ProcDef]] = {}
        with self._lock:
            cached = list(self._cached.items())
        for uri, sr in cached:
            if sr.dialect_hint == "f5-irules" and sr.analysis.all_procs:
                result[uri] = dict(sr.analysis.all_procs)
        return result

    @property
    def irules_rule_init_vars(self) -> dict[str, list[RuleInitExport]]:
        """Return uri -> [RuleInitExport] for all cached iRules files."""
        result: dict[str, list[RuleInitExport]] = {}
        with self._lock:
            cached = list(self._cached.items())
        for uri, sr in cached:
            if sr.rule_init_exports:
                result[uri] = list(sr.rule_init_exports)
        return result

    @property
    def bigip_configs(self) -> dict[str, BigipConfig]:
        """Return uri -> BigipConfig for all cached BIG-IP configuration files."""
        with self._lock:
            return dict(self._bigip_configs)

    @property
    def merged_bigip_config(self) -> BigipConfig | None:
        """Return a merged BigipConfig from all scanned conf files, or None.

        Uses :meth:`BigipConfig.merge` so every dict-valued kind on the
        dataclass — not just the v1 ten — flows into the merged view.
        Adding a new kind to ``BigipConfig`` no longer requires also
        editing this scanner; it just shows up.
        """
        with self._lock:
            configs = list(self._bigip_configs.values())
        if not configs:
            return None
        merged = BigipConfig()
        for cfg in configs:
            merged.merge(cfg)
        return merged

    def parse_bigip_source(self, uri: str, source: str) -> BigipConfig | None:
        """Parse a BIG-IP config from source text and cache the result."""
        try:
            config = parse_bigip_conf(source)
            with self._lock:
                self._bigip_configs[uri] = config
            return config
        except Exception:
            log.debug("Scanner: failed to parse bigip config %s", uri, exc_info=True)
            return None

    def remove_bigip_config(self, uri: str) -> None:
        with self._lock:
            self._bigip_configs.pop(uri, None)

    # APL model caching

    @property
    def apl_models(self) -> dict[str, AplModel]:
        """Return uri -> AplModel for all cached APL presentation files."""
        with self._lock:
            return dict(self._apl_models)

    def parse_apl_source(
        self, uri: str, source: str, base_dir: str | None = None
    ) -> AplModel | None:
        """Parse an APL presentation source and cache the result."""
        try:
            model = resolve_apl_includes(source, base_dir)
            with self._lock:
                self._apl_models[uri] = model
            return model
        except Exception:
            log.debug("Scanner: failed to parse APL %s", uri, exc_info=True)
            return None

    def remove_apl_model(self, uri: str) -> None:
        with self._lock:
            self._apl_models.pop(uri, None)

    def find_sibling_apl(self, uri: str) -> AplModel | None:
        """Find a cached APL model from the same directory as *uri*."""
        # Extract directory from the URI
        if "/" in uri:
            dir_part = uri.rsplit("/", 1)[0]
        else:
            return None
        with self._lock:
            apl_models = list(self._apl_models.items())
        for apl_uri, model in apl_models:
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
        with self._lock:
            cached = list(self._cached.items())
        for cached_uri, sr in cached:
            if "/" not in cached_uri:
                continue
            if cached_uri.rsplit("/", 1)[0] != dir_part:
                continue
            if sr.dialect_hint == "f5-iapps":
                return cached_uri
        return None

    def _parse_bigip_file(
        self,
        full_path: str,
        bigip_configs: dict[str, BigipConfig] | None = None,
    ) -> BigipConfig | None:
        """Parse a BIG-IP configuration file and cache the result."""
        uri = path_to_uri(full_path)
        try:
            source = Path(full_path).read_text(encoding="utf-8", errors="replace")
            config = parse_bigip_conf(source)
            if bigip_configs is None:
                with self._lock:
                    self._bigip_configs[uri] = config
            else:
                bigip_configs[uri] = config
            log.debug("Scanner: parsed bigip config %s", full_path)
            return config
        except Exception:
            log.debug("Scanner: failed to parse bigip config %s", full_path, exc_info=True)
            return None

    def _run_analysis(self, full_path: str, ext: str) -> ScanResult | None:
        """Run signature-only analysis for a single file without caching."""
        uri = path_to_uri(full_path)
        try:
            source = Path(full_path).read_text(encoding="utf-8", errors="replace")
            # Background-scanned files only contribute cross-file signals:
            # proc/class signatures, package requires, source targets, and
            # aliases. ``extract_signatures`` skips diagnostics, the
            # optimiser, variable-reference tracking, lowering, and every
            # other stage ``analyse()`` runs, which is both far faster and
            # far lighter. Full analysis is still run on ``didOpen``.
            result = extract_signatures(source)
            dialect = _dialect_from_ext(ext)
            rule_init_exports: list[RuleInitExport] = []
            if dialect == "f5-irules":
                from compiler.irules_flow import extract_rule_init_vars

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
            with self._lock:
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
