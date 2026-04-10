"""tcltk-pkgs registry client with TTL-based caching.

The upstream registry at ``github.com/tcltk-pkgs/registry`` ships a
single ``packages.json`` with discovery metadata — name, description,
sources, tags.  This client fetches it, caches it under
``<cache_dir>/tclpkg/registry/``, and respects a 24-hour TTL.

Conditional GET (``If-None-Match`` / ``304 Not Modified``) avoids
re-downloading when the remote hasn't changed.  Offline mode
(``--offline``) reads from the cache only.
"""

from __future__ import annotations

import json
import logging
import urllib.error
import urllib.request
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from pathlib import Path

from .errors import RegistryError

log = logging.getLogger(__name__)

_DEFAULT_REGISTRY_URL = "https://raw.githubusercontent.com/tcltk-pkgs/registry/main/packages.json"

_TTL = timedelta(hours=24)


@dataclass
class RegistrySource:
    """One source entry inside a registry package."""

    url: str = ""
    method: str = ""
    web: str = ""
    author: str = ""
    license: str = ""
    extension: bool = False


@dataclass
class RegistryEntry:
    """One package entry from packages.json."""

    name: str = ""
    description: str = ""
    tags: list[str] = field(default_factory=list)
    sources: list[RegistrySource] = field(default_factory=list)


def _parse_entries(data: list) -> list[RegistryEntry]:
    entries: list[RegistryEntry] = []
    for item in data:
        if not isinstance(item, dict):
            continue
        sources: list[RegistrySource] = []
        for s in item.get("sources", []):
            if isinstance(s, dict):
                sources.append(
                    RegistrySource(
                        url=s.get("url", ""),
                        method=s.get("method", ""),
                        web=s.get("web", ""),
                        author=s.get("author", ""),
                        license=s.get("license", ""),
                        extension=bool(s.get("extension", False)),
                    )
                )
        entries.append(
            RegistryEntry(
                name=item.get("name", ""),
                description=item.get("description", ""),
                tags=list(item.get("tags", [])),
                sources=sources,
            )
        )
    return entries


class RegistryClient:
    """Fetch, cache, and query the tcltk-pkgs package registry."""

    def __init__(
        self,
        cache_dir: Path,
        *,
        registry_url: str = _DEFAULT_REGISTRY_URL,
        offline: bool = False,
    ) -> None:
        self._dir = cache_dir / "tclpkg" / "registry"
        self._url = registry_url
        self._offline = offline
        self._entries: list[RegistryEntry] | None = None

    def _cached_path(self) -> Path:
        return self._dir / "packages.json"

    def _etag_path(self) -> Path:
        return self._dir / "packages.json.etag"

    def _fetched_path(self) -> Path:
        return self._dir / "fetched_at"

    def _is_fresh(self) -> bool:
        fetched = self._fetched_path()
        if not fetched.is_file():
            return False
        try:
            ts = datetime.fromisoformat(fetched.read_text().strip())
        except (ValueError, OSError):
            return False
        return datetime.now(timezone.utc) - ts < _TTL

    def _load_cached(self) -> list[RegistryEntry]:
        cached = self._cached_path()
        if not cached.is_file():
            raise RegistryError(
                "no cached registry and offline mode is active",
                hint="run 'tcl pkg search' with network access first",
            )
        data = json.loads(cached.read_text(encoding="utf-8"))
        return _parse_entries(data)

    def _write_cache(self, body: bytes, etag: str) -> None:
        self._dir.mkdir(parents=True, exist_ok=True)
        self._cached_path().write_bytes(body)
        self._etag_path().write_text(etag, encoding="utf-8")
        self._fetched_path().write_text(datetime.now(timezone.utc).isoformat(), encoding="utf-8")

    def fetch(self, *, force: bool = False) -> list[RegistryEntry]:
        """Return all registry entries, fetching/caching as needed."""
        if self._entries is not None and not force:
            return self._entries

        cached = self._cached_path()

        if self._offline:
            self._entries = self._load_cached()
            return self._entries

        if not force and self._is_fresh() and cached.is_file():
            data = json.loads(cached.read_text(encoding="utf-8"))
            self._entries = _parse_entries(data)
            return self._entries

        # Conditional GET.
        req = urllib.request.Request(self._url, headers={"User-Agent": "tclpkg/0.1"})
        etag_path = self._etag_path()
        if etag_path.is_file():
            req.add_header("If-None-Match", etag_path.read_text().strip())

        try:
            with urllib.request.urlopen(req, timeout=30) as resp:
                body = resp.read()
                new_etag = resp.headers.get("ETag", "")
        except urllib.error.HTTPError as exc:
            if exc.code == 304:
                # Not modified — touch the timestamp and use cache.
                self._fetched_path().parent.mkdir(parents=True, exist_ok=True)
                self._fetched_path().write_text(
                    datetime.now(timezone.utc).isoformat(), encoding="utf-8"
                )
                return self._load_cached()
            if cached.is_file():
                log.warning("Registry fetch failed (HTTP %s); using stale cache.", exc.code)
                self._entries = self._load_cached()
                return self._entries
            raise RegistryError(f"registry fetch failed: HTTP {exc.code}") from exc
        except (urllib.error.URLError, TimeoutError, OSError) as exc:
            if cached.is_file():
                log.warning("Registry fetch failed; using stale cache.")
                self._entries = self._load_cached()
                return self._entries
            raise RegistryError(
                f"cannot reach registry at {self._url}: {exc}",
                hint="check your network connection or use --offline",
            ) from exc

        self._write_cache(body, new_etag)
        data = json.loads(body)
        self._entries = _parse_entries(data)
        return self._entries

    def search(self, query: str) -> list[RegistryEntry]:
        """Search cached entries by name/description/tag (case-insensitive)."""
        entries = self.fetch()
        q = query.lower()
        results: list[RegistryEntry] = []
        for e in entries:
            if (
                q in e.name.lower()
                or q in e.description.lower()
                or any(q in t.lower() for t in e.tags)
            ):
                results.append(e)
        return results

    def lookup(self, name: str) -> RegistryEntry | None:
        """Find an entry by exact package name."""
        entries = self.fetch()
        for e in entries:
            if e.name == name:
                return e
        return None


__all__ = ["RegistryClient", "RegistryEntry", "RegistrySource"]
