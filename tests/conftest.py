from __future__ import annotations

import logging
import subprocess
from collections.abc import Generator
from pathlib import Path

import pytest

logger = logging.getLogger(__name__)

# Tcl source tree management
#
# Version → (GitHub tag, local directory name)
_TCL_VERSIONS: dict[str, tuple[str, str]] = {
    "9.0": ("core-9-0-3", "tcl9.0.3"),
    "8.6": ("core-8-6-16", "tcl8.6.16"),
    "8.5": ("core-8-5-19", "tcl8.5.19"),
    "8.4": ("core-8-4-20", "tcl8.4.20"),
}

_GITHUB_REPO = "https://github.com/tcltk/tcl.git"

_TMP_DIR = Path(__file__).resolve().parent.parent / "tmp"


def ensure_tcl_source(version: str = "9.0") -> Path:
    """Ensure Tcl test files are available and return the tests/ directory.

    Uses ``git sparse-checkout`` to fetch only the ``tests/`` and
    ``library/`` directories from the tcltk/tcl GitHub repository,
    keeping the download small (~11 MB instead of ~100 MB+).

    Returns the path to ``tmp/tcl<full_version>/tests/``.
    Calls ``pytest.skip`` on network or git failure so tests degrade
    gracefully in offline environments.
    """
    tag, dirname = _TCL_VERSIONS[version]
    target = _TMP_DIR / dirname

    if (target / "tests").is_dir():
        return target / "tests"

    _TMP_DIR.mkdir(parents=True, exist_ok=True)

    logger.info("Fetching Tcl %s tests from GitHub (sparse checkout) ...", version)

    try:
        subprocess.run(
            [
                "git",
                "clone",
                "--depth",
                "1",
                "--filter=blob:none",
                "--sparse",
                "--branch",
                tag,
                _GITHUB_REPO,
                str(target),
            ],
            check=True,
            capture_output=True,
            timeout=120,
        )
        subprocess.run(
            ["git", "sparse-checkout", "set", "tests", "library"],
            cwd=target,
            check=True,
            capture_output=True,
            timeout=60,
        )
    except (subprocess.CalledProcessError, subprocess.TimeoutExpired, FileNotFoundError) as exc:
        pytest.skip(f"Cannot fetch Tcl source from GitHub: {exc}")

    if not (target / "tests").is_dir():
        pytest.skip(f"Sparse checkout did not produce {target}/tests/")

    return target / "tests"


@pytest.fixture(autouse=True)
def _reset_signature_profile() -> Generator[None, None, None]:
    """Keep command-profile global state isolated between tests."""
    from core.commands.registry.runtime import configure_signatures

    configure_signatures(
        dialect="tcl8.6",
        extra_commands=[],
    )
    yield
    configure_signatures(
        dialect="tcl8.6",
        extra_commands=[],
    )
