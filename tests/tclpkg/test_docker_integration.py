"""Slow integration tests for ``tclpkg.docker`` — actually builds and runs Docker images.

These tests require a working Docker daemon. They are skipped automatically
when Docker is unavailable (e.g. in CI without Docker-in-Docker, or on
machines without Docker installed).

Run explicitly with::

    pytest tests/tclpkg/test_docker_integration.py -v

Or as part of ``make test-slow``.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import textwrap
from pathlib import Path

import pytest

from tooling.tclpkg.docker import (
    DockerfileSpec,
    generate_dockerfile,
)

pytestmark = pytest.mark.slow


def _docker_available() -> bool:
    """Return True if the ``docker`` CLI is on PATH and the daemon responds."""
    if not shutil.which("docker"):
        return False
    try:
        result = subprocess.run(
            ["docker", "info"],
            capture_output=True,
            timeout=10,
        )
        return result.returncode == 0
    except (subprocess.TimeoutExpired, OSError):
        return False


skip_no_docker = pytest.mark.skipif(
    not _docker_available(),
    reason="Docker daemon not available",
)


def _build_image(build_dir: Path, tag: str, *, timeout: int = 300) -> subprocess.CompletedProcess:
    """Run ``docker build`` and return the completed process."""
    return subprocess.run(
        ["docker", "build", "-t", tag, "."],
        cwd=build_dir,
        capture_output=True,
        text=True,
        timeout=timeout,
    )


def _run_container(
    tag: str,
    command: list[str] | None = None,
    *,
    timeout: int = 30,
) -> subprocess.CompletedProcess:
    """Run a container from *tag* and return the completed process."""
    cmd = ["docker", "run", "--rm", tag]
    if command:
        cmd.extend(command)
    return subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        timeout=timeout,
    )


def _remove_image(tag: str) -> None:
    """Best-effort removal of the Docker image."""
    subprocess.run(
        ["docker", "rmi", "-f", tag],
        capture_output=True,
        timeout=30,
    )


def _setup_project(tmp_path: Path, spec: DockerfileSpec, *, script: str | None = None) -> Path:
    """Write a Dockerfile and optional Tcl script into *tmp_path*."""
    dockerfile = tmp_path / "Dockerfile"
    dockerfile.write_text(generate_dockerfile(spec), encoding="utf-8")
    if script:
        (tmp_path / "main.tcl").write_text(script, encoding="utf-8")
    return tmp_path


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


@skip_no_docker
class TestDockerBuildDebian:
    """Build and run a Tcl container on Debian."""

    TAG = "tcl-lsp-test-debian-86"

    def test_build_and_run_hello(self, tmp_path: Path) -> None:
        spec = DockerfileSpec(
            base_image="debian:bookworm-slim",
            tcl_version="8.6",
            entrypoint="main.tcl",
            install_packages=False,
        )
        _setup_project(
            tmp_path,
            spec,
            script='puts "hello from tcl [info patchlevel]"',
        )

        result = _build_image(tmp_path, self.TAG)
        try:
            assert result.returncode == 0, f"build failed:\n{result.stderr}"

            run = _run_container(self.TAG)
            assert run.returncode == 0, f"run failed:\n{run.stderr}"
            assert "hello from tcl" in run.stdout
            assert "8.6" in run.stdout
        finally:
            _remove_image(self.TAG)

    def test_tclsh_is_on_path(self, tmp_path: Path) -> None:
        spec = DockerfileSpec(
            base_image="debian:bookworm-slim",
            tcl_version="8.6",
            install_packages=False,
            copy_project=False,
        )
        _setup_project(tmp_path, spec)

        result = _build_image(tmp_path, self.TAG)
        try:
            assert result.returncode == 0, f"build failed:\n{result.stderr}"

            run = _run_container(
                self.TAG,
                ["sh", "-c", 'echo "puts [info patchlevel]" | tclsh'],
            )
            assert run.returncode == 0, f"run failed:\n{run.stderr}"
            assert run.stdout.strip().startswith("8.6")
        finally:
            _remove_image(self.TAG)


@skip_no_docker
class TestDockerBuildAlpine:
    """Build and run a Tcl container on Alpine."""

    TAG = "tcl-lsp-test-alpine-86"

    def test_build_and_run_hello(self, tmp_path: Path) -> None:
        spec = DockerfileSpec(
            base_image="alpine:3.19",
            tcl_version="8.6",
            entrypoint="main.tcl",
            install_packages=False,
        )
        _setup_project(
            tmp_path,
            spec,
            script='puts "hello from alpine tcl [info patchlevel]"',
        )

        result = _build_image(tmp_path, self.TAG)
        try:
            assert result.returncode == 0, f"build failed:\n{result.stderr}"

            run = _run_container(self.TAG)
            assert run.returncode == 0, f"run failed:\n{run.stderr}"
            assert "hello from alpine tcl" in run.stdout
            assert "8.6" in run.stdout
        finally:
            _remove_image(self.TAG)

    def test_small_image_size(self, tmp_path: Path) -> None:
        """Alpine images with Tcl should be reasonably small."""
        spec = DockerfileSpec(
            base_image="alpine:3.19",
            tcl_version="8.6",
            install_packages=False,
            copy_project=False,
        )
        _setup_project(tmp_path, spec)

        result = _build_image(tmp_path, self.TAG)
        try:
            assert result.returncode == 0

            inspect = subprocess.run(
                ["docker", "image", "inspect", self.TAG, "--format", "{{.Size}}"],
                capture_output=True,
                text=True,
                timeout=10,
            )
            assert inspect.returncode == 0
            size_bytes = int(inspect.stdout.strip())
            # Alpine + Tcl 8.6 should be under 50MB.
            assert size_bytes < 50_000_000, f"image too large: {size_bytes / 1_000_000:.1f}MB"
        finally:
            _remove_image(self.TAG)


@skip_no_docker
class TestDockerBuildFromSource:
    """Build Tcl from source (9.0) — slower but validates the recipe works."""

    TAG = "tcl-lsp-test-debian-90"

    def test_build_tcl_90_from_source(self, tmp_path: Path) -> None:
        spec = DockerfileSpec(
            base_image="debian:bookworm-slim",
            tcl_version="9.0",
            entrypoint="main.tcl",
            install_packages=False,
        )
        _setup_project(
            tmp_path,
            spec,
            script='puts "tcl9: [info patchlevel]"',
        )

        # Building from source takes longer.
        result = _build_image(tmp_path, self.TAG, timeout=600)
        try:
            assert result.returncode == 0, f"build failed:\n{result.stderr}"

            run = _run_container(self.TAG)
            assert run.returncode == 0, f"run failed:\n{run.stderr}"
            assert "tcl9:" in run.stdout
            assert "9.0" in run.stdout
        finally:
            _remove_image(self.TAG)


@skip_no_docker
class TestDockerVenv:
    """Verify that venv creation inside Docker works.

    Note: These tests require a released tcl CLI pyz that includes the
    ``venv`` verb.  Until the package-manager branch ships in a release,
    the pyz downloaded inside the container won't have ``venv`` and
    Docker builds that use ``create_venv=True`` will fail.  We mark
    these tests as ``xfail`` until the next release.
    """

    TAG = "tcl-lsp-test-venv"

    @pytest.mark.xfail(
        reason="Released tcl pyz (v1.6.0) does not yet include the venv verb",
        strict=False,
    )
    def test_venv_sets_tcllibpath(self, tmp_path: Path) -> None:
        spec = DockerfileSpec(
            base_image="debian:bookworm-slim",
            tcl_version="8.6",
            create_venv=True,
            install_packages=False,
            copy_project=False,
        )
        _setup_project(tmp_path, spec)

        result = _build_image(tmp_path, self.TAG)
        try:
            assert result.returncode == 0, f"build failed:\n{result.stderr}"

            run = _run_container(
                self.TAG,
                ["sh", "-c", 'echo "$TCLLIBPATH"'],
            )
            assert run.returncode == 0
            assert "/app/.venv/lib" in run.stdout
        finally:
            _remove_image(self.TAG)


@skip_no_docker
class TestDockerExtraPackages:
    """Verify extra OS packages get installed."""

    TAG = "tcl-lsp-test-extras"

    def test_extra_package_installed(self, tmp_path: Path) -> None:
        spec = DockerfileSpec(
            base_image="debian:bookworm-slim",
            tcl_version="8.6",
            extra_packages=["curl"],
            install_packages=False,
            copy_project=False,
        )
        _setup_project(tmp_path, spec)

        result = _build_image(tmp_path, self.TAG)
        try:
            assert result.returncode == 0, f"build failed:\n{result.stderr}"

            run = _run_container(
                self.TAG,
                ["curl", "--version"],
            )
            assert run.returncode == 0
            assert "curl" in run.stdout
        finally:
            _remove_image(self.TAG)


@skip_no_docker
class TestDockerProjectBundle:
    """Verify project files are COPYed and can run inside the container."""

    TAG = "tcl-lsp-test-project"

    def test_project_files_copied(self, tmp_path: Path) -> None:
        spec = DockerfileSpec(
            base_image="alpine:3.19",
            tcl_version="8.6",
            entrypoint="main.tcl",
            install_packages=False,
        )
        script = textwrap.dedent("""\
            # Simple Tcl project that reads a data file.
            set f [open "data.txt" r]
            set content [read $f]
            close $f
            puts "data: [string trim $content]"
        """)
        _setup_project(tmp_path, spec, script=script)
        (tmp_path / "data.txt").write_text("hello-from-file\n")

        result = _build_image(tmp_path, self.TAG)
        try:
            assert result.returncode == 0, f"build failed:\n{result.stderr}"

            run = _run_container(self.TAG)
            assert run.returncode == 0, f"run failed:\n{run.stderr}"
            assert "data: hello-from-file" in run.stdout
        finally:
            _remove_image(self.TAG)

    def test_workdir_is_correct(self, tmp_path: Path) -> None:
        spec = DockerfileSpec(
            base_image="alpine:3.19",
            tcl_version="8.6",
            workdir="/opt/myapp",
            install_packages=False,
            copy_project=False,
        )
        _setup_project(tmp_path, spec)

        result = _build_image(tmp_path, self.TAG)
        try:
            assert result.returncode == 0

            run = _run_container(self.TAG, ["pwd"])
            assert run.returncode == 0
            assert run.stdout.strip() == "/opt/myapp"
        finally:
            _remove_image(self.TAG)


@skip_no_docker
class TestDockerLabelsAndEnv:
    """Verify labels and environment variables appear in the image."""

    TAG = "tcl-lsp-test-labels"

    def test_labels_applied(self, tmp_path: Path) -> None:
        spec = DockerfileSpec(
            base_image="alpine:3.19",
            tcl_version="8.6",
            labels={"org.tcl-lsp.test": "yes", "version": "1.0"},
            install_packages=False,
            copy_project=False,
        )
        _setup_project(tmp_path, spec)

        result = _build_image(tmp_path, self.TAG)
        try:
            assert result.returncode == 0

            inspect = subprocess.run(
                ["docker", "image", "inspect", self.TAG],
                capture_output=True,
                text=True,
                timeout=10,
            )
            assert inspect.returncode == 0
            data = json.loads(inspect.stdout)
            labels = data[0].get("Config", {}).get("Labels", {})
            assert labels.get("org.tcl-lsp.test") == "yes"
            assert labels.get("version") == "1.0"
        finally:
            _remove_image(self.TAG)

    def test_env_vars_set(self, tmp_path: Path) -> None:
        spec = DockerfileSpec(
            base_image="alpine:3.19",
            tcl_version="8.6",
            env={"MY_APP_NAME": "testapp"},
            install_packages=False,
            copy_project=False,
        )
        _setup_project(tmp_path, spec)

        result = _build_image(tmp_path, self.TAG)
        try:
            assert result.returncode == 0

            run = _run_container(
                self.TAG,
                ["sh", "-c", 'echo "$MY_APP_NAME"'],
            )
            assert run.returncode == 0
            assert "testapp" in run.stdout
        finally:
            _remove_image(self.TAG)
