from __future__ import annotations

import json
import logging
import subprocess
from collections.abc import Generator
from pathlib import Path
from typing import Any

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


# Module-level flag flipped to True by :func:`pytest_configure` below
# when the session runs with ``--tcl9-required``. Kept module-level
# (not a fixture) so ``ensure_tcl_source`` can consult it from
# non-fixture call sites such as test-class setup helpers.
TCL9_REQUIRED: bool = False


def _missing_source_action(message: str) -> None:
    """Skip the current test, or fail it when ``TCL9_REQUIRED`` is set."""
    if TCL9_REQUIRED:
        pytest.fail(message)
    pytest.skip(message)


def ensure_tcl_source(version: str = "9.0") -> Path:
    """Ensure Tcl test files are available and return the tests/ directory.

    Uses ``git sparse-checkout`` to fetch only the ``tests/`` and
    ``library/`` directories from the tcltk/tcl GitHub repository,
    keeping the download small (~11 MB instead of ~100 MB+).

    Returns the path to ``tmp/tcl<full_version>/tests/``.
    Calls ``pytest.skip`` on network or git failure so tests degrade
    gracefully in offline environments, unless the session was
    invoked with ``--tcl9-required`` (then the skip becomes a hard
    ``pytest.fail``).
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
        _missing_source_action(f"Cannot fetch Tcl source from GitHub: {exc}")

    if not (target / "tests").is_dir():
        _missing_source_action(f"Sparse checkout did not produce {target}/tests/")

    return target / "tests"


# ---------------------------------------------------------------------------
# Tcl 9 correctness-harness command-line options
# ---------------------------------------------------------------------------


def pytest_addoption(parser: pytest.Parser) -> None:
    group = parser.getgroup("tcl9", "Tcl 9 correctness harness")
    group.addoption(
        "--tcl9-report",
        action="store",
        dest="tcl9_report",
        default=None,
        help="Write per-file Tcl 9 test results as JSON to this path.",
    )
    group.addoption(
        "--tcl9-required",
        action="store_true",
        dest="tcl9_required",
        default=False,
        help=(
            "Fail instead of skipping when the upstream Tcl 9 source "
            "cannot be fetched (no network or no git)."
        ),
    )


def pytest_configure(config: pytest.Config) -> None:
    global TCL9_REQUIRED
    if config.getoption("tcl9_required", default=False):
        TCL9_REQUIRED = True


_RESULTS_KEY = "_tcl9_results"


def record_tcl9_result(config: pytest.Config, entry: dict[str, Any]) -> None:
    """Record one per-test-file result onto the session config."""
    bucket = getattr(config, _RESULTS_KEY, None)
    if bucket is None:
        bucket = []
        setattr(config, _RESULTS_KEY, bucket)
    bucket.append(entry)


def pytest_sessionfinish(session: pytest.Session, exitstatus: int) -> None:
    path = session.config.getoption("tcl9_report", default=None)
    if not path:
        return
    bucket = getattr(session.config, _RESULTS_KEY, None) or []
    out_path = Path(path)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(
        json.dumps(bucket, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


@pytest.fixture(autouse=True)
def _reset_signature_profile() -> Generator[None, None, None]:
    """Keep dialect ContextVars isolated between tests.

    Post-#407 the active dialect is held in a ContextVar rather than a
    process-wide global, and ``SIGNATURES`` is a cached per-(dialect,
    extras) view.  Each test starts and ends with the default profile
    (``tcl8.6`` + no extras) so that ad-hoc ``configure_signatures`` calls
    or escaped ``dialect_scope`` mutations from one test don't leak into
    the next.
    """
    from compiler.registry.runtime import configure_signatures

    configure_signatures(
        dialect="tcl8.6",
        extra_commands=[],
    )
    yield
    configure_signatures(
        dialect="tcl8.6",
        extra_commands=[],
    )
