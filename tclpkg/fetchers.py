"""Package source fetchers — tarball, git, and local path.

Each fetcher downloads/copies a package source into a temporary directory
and returns its path.  The caller (``install`` workflow) then passes the
directory to ``cas.store()`` for integrity hashing and CAS ingestion.

All network I/O uses stdlib ``urllib.request`` (no new dependency).
Git uses the host ``git`` binary via ``subprocess`` (same pattern as
``tests/conftest.py``).
"""

from __future__ import annotations

import logging
import shutil
import subprocess
import tarfile
import tempfile
import urllib.error
import urllib.request
import zipfile
from pathlib import Path

from .errors import TclPkgError

log = logging.getLogger(__name__)


class FetchError(TclPkgError):
    """A package source could not be fetched."""

    category = "fetch"


def fetch_tarball(url: str, dest: Path, *, timeout: int = 60) -> None:
    """Download and extract a tarball (or zip) from *url* into *dest*.

    Supports ``.tar.gz``, ``.tgz``, ``.tar.bz2``, ``.tar.xz``, and ``.zip``.
    The archive's top-level directory (if singular) is stripped so the
    extracted files land directly inside *dest*.
    """
    dest.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(suffix=".archive", delete=False) as tmp:
        tmp_path = Path(tmp.name)
    try:
        log.info("Fetching %s", url)
        req = urllib.request.Request(url, headers={"User-Agent": "tclpkg/0.1"})
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            tmp_path.write_bytes(resp.read())

        # Extract into a staging dir, then move contents into dest.
        staging = dest / ".staging"
        staging.mkdir(exist_ok=True)

        lower = url.lower()
        if lower.endswith(".zip"):
            with zipfile.ZipFile(tmp_path) as zf:
                # Zip Slip protection: reject absolute paths and members
                # that escape the staging directory.
                staging_resolved = staging.resolve()
                for member in zf.namelist():
                    if Path(member).is_absolute():
                        raise FetchError(f"zip member has absolute path: {member}")
                    resolved = (staging / member).resolve()
                    try:
                        resolved.relative_to(staging_resolved)
                    except ValueError:
                        raise FetchError(f"zip member escapes target directory: {member}")
                zf.extractall(staging)
        else:
            with tarfile.open(tmp_path) as tf:
                tf.extractall(staging, filter="data")

        # Strip singular top-level directory (common convention).
        children = list(staging.iterdir())
        if len(children) == 1 and children[0].is_dir():
            # Move contents of the single child into dest.
            for item in children[0].iterdir():
                shutil.move(str(item), str(dest / item.name))
            shutil.rmtree(staging)
        else:
            for item in staging.iterdir():
                shutil.move(str(item), str(dest / item.name))
            staging.rmdir()
    except (urllib.error.URLError, OSError, tarfile.TarError, zipfile.BadZipFile) as exc:
        raise FetchError(f"failed to fetch tarball from {url}: {exc}") from exc
    finally:
        tmp_path.unlink(missing_ok=True)


def fetch_git(url: str, dest: Path, *, rev: str | None = None, timeout: int = 120) -> str:
    """Clone a git repository into *dest* and return the resolved commit SHA.

    If *rev* is a tag or branch name, clones at that ref.  If it looks
    like a full SHA, does a shallow clone then checks out the commit.
    The ``.git/`` directory is removed after cloning to keep the
    worktree clean for integrity hashing.
    """
    dest.mkdir(parents=True, exist_ok=True)
    git = shutil.which("git")
    if git is None:
        raise FetchError("git not found on PATH; install git to fetch git-based packages")

    # Strip git+ prefix if present (e.g. git+https://...).
    if url.startswith("git+"):
        url = url[4:]
    # Strip @rev suffix if embedded in URL, but only when it looks like
    # a ref marker (after .git or after a path segment), not an SSH user@
    # prefix like git@github.com:org/repo.git.
    if rev is None and "@" in url:
        # Only split on the last @ if it appears after .git or after a /
        last_at = url.rfind("@")
        prefix = url[:last_at]
        if "/" in prefix or prefix.endswith(".git"):
            rev = url[last_at + 1 :]
            url = prefix

    clone_cmd: list[str] = [git, "clone", "--depth", "1"]
    if rev:
        clone_cmd += ["--branch", rev]
    clone_cmd += [url, str(dest)]

    try:
        subprocess.run(
            clone_cmd,
            check=True,
            capture_output=True,
            timeout=timeout,
        )
    except subprocess.CalledProcessError as exc:
        stderr = exc.stderr.decode(errors="replace").strip()
        raise FetchError(f"git clone failed: {stderr}") from exc
    except FileNotFoundError as exc:
        raise FetchError(f"git not found: {exc}") from exc

    # Get the resolved SHA.
    try:
        result = subprocess.run(
            [git, "rev-parse", "HEAD"],
            cwd=dest,
            check=True,
            capture_output=True,
            timeout=10,
        )
        sha = result.stdout.decode().strip()
    except Exception:
        sha = ""

    # Strip .git directory.
    git_dir = dest / ".git"
    if git_dir.is_dir():
        shutil.rmtree(git_dir)

    return sha


def fetch_path(source: str, dest: Path) -> None:
    """Copy a local directory into *dest*.

    Used for ``replace`` directives that point at a local checkout.
    """
    src = Path(source)
    if not src.is_dir():
        raise FetchError(f"local path not found: {source}")
    dest.mkdir(parents=True, exist_ok=True)
    shutil.copytree(src, dest, dirs_exist_ok=True)


__all__ = ["FetchError", "fetch_git", "fetch_path", "fetch_tarball"]
