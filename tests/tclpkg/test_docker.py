"""Tests for ``tclpkg.docker`` — Dockerfile generation for Tcl projects."""

from __future__ import annotations

from pathlib import Path

import pytest

from tooling.tclpkg.docker import (
    DEFAULT_BASE_IMAGE,
    DEFAULT_TCL_VERSION,
    SUPPORTED_TCL_VERSIONS,
    DockerError,
    DockerfileSpec,
    available_recipes,
    detect_image_family,
    generate_dockerfile,
    python_install_recipe,
    tcl_cli_download_url,
    tcl_install_recipe,
    write_dockerfile,
)

# ---------------------------------------------------------------------------
# detect_image_family
# ---------------------------------------------------------------------------


class TestDetectImageFamily:
    def test_debian(self) -> None:
        assert detect_image_family("debian:bookworm-slim") == "debian"

    def test_ubuntu(self) -> None:
        assert detect_image_family("ubuntu:22.04") == "debian"

    def test_alpine(self) -> None:
        assert detect_image_family("alpine:3.19") == "alpine"

    def test_fedora(self) -> None:
        assert detect_image_family("fedora:39") == "redhat"

    def test_centos(self) -> None:
        assert detect_image_family("centos:stream9") == "redhat"

    def test_rockylinux(self) -> None:
        assert detect_image_family("rockylinux:9") == "redhat"

    def test_almalinux(self) -> None:
        assert detect_image_family("almalinux:9") == "redhat"

    def test_amazonlinux(self) -> None:
        assert detect_image_family("amazonlinux:2023") == "redhat"

    def test_buildpack_deps(self) -> None:
        assert detect_image_family("buildpack-deps:bookworm") == "debian"

    def test_slim_variant(self) -> None:
        assert detect_image_family("slim:latest") == "debian"

    def test_unknown_defaults_to_debian(self) -> None:
        assert detect_image_family("custom-image:latest") == "debian"

    def test_case_insensitive(self) -> None:
        assert detect_image_family("Alpine:3.19") == "alpine"
        assert detect_image_family("UBUNTU:22.04") == "debian"

    def test_fully_qualified_docker_io(self) -> None:
        assert detect_image_family("docker.io/library/alpine:3.19") == "alpine"
        assert detect_image_family("docker.io/library/ubuntu:22.04") == "debian"
        assert detect_image_family("docker.io/library/fedora:39") == "redhat"

    def test_fully_qualified_quay_io(self) -> None:
        assert detect_image_family("quay.io/fedora/fedora:39") == "redhat"

    def test_fully_qualified_ghcr(self) -> None:
        assert detect_image_family("ghcr.io/myorg/debian:bookworm") == "debian"

    def test_registry_with_port(self) -> None:
        assert detect_image_family("registry.example.com:5000/alpine:3.19") == "alpine"

    def test_three_part_path(self) -> None:
        assert detect_image_family("myregistry.io/myns/ubuntu:22.04") == "debian"


# ---------------------------------------------------------------------------
# tcl_install_recipe
# ---------------------------------------------------------------------------


class TestTclInstallRecipe:
    def test_debian_86(self) -> None:
        recipe = tcl_install_recipe("debian:bookworm", "8.6")
        assert "apt-get" in recipe
        assert "tcl8.6" in recipe

    def test_alpine_86(self) -> None:
        recipe = tcl_install_recipe("alpine:3.19", "8.6")
        assert "apk" in recipe

    def test_redhat_86(self) -> None:
        recipe = tcl_install_recipe("fedora:39", "8.6")
        assert "dnf" in recipe

    def test_debian_90_from_source(self) -> None:
        recipe = tcl_install_recipe("debian:bookworm", "9.0")
        assert "curl" in recipe
        assert "configure" in recipe
        assert "9.0" in recipe

    def test_alpine_84_from_source(self) -> None:
        recipe = tcl_install_recipe("alpine:3.19", "8.4")
        assert "8.4.20" in recipe
        assert "configure" in recipe

    def test_all_versions_have_recipes(self) -> None:
        for version in SUPPORTED_TCL_VERSIONS:
            for image in ("debian:bookworm", "alpine:3.19", "fedora:39"):
                recipe = tcl_install_recipe(image, version)
                assert recipe, f"no recipe for {image} + Tcl {version}"

    def test_unsupported_version_raises(self) -> None:
        with pytest.raises(DockerError, match="unsupported Tcl version"):
            tcl_install_recipe("debian:bookworm", "7.0")

    def test_ubuntu_uses_debian_recipe(self) -> None:
        deb = tcl_install_recipe("debian:bookworm", "8.6")
        ubu = tcl_install_recipe("ubuntu:22.04", "8.6")
        assert deb == ubu


# ---------------------------------------------------------------------------
# python_install_recipe
# ---------------------------------------------------------------------------


class TestPythonInstallRecipe:
    def test_debian(self) -> None:
        recipe = python_install_recipe("debian:bookworm")
        assert "python3" in recipe
        assert "apt-get" in recipe

    def test_alpine(self) -> None:
        recipe = python_install_recipe("alpine:3.19")
        assert "python3" in recipe
        assert "apk" in recipe

    def test_redhat(self) -> None:
        recipe = python_install_recipe("fedora:39")
        assert "python3" in recipe
        assert "dnf" in recipe

    def test_ubuntu_uses_debian(self) -> None:
        deb = python_install_recipe("debian:bookworm")
        ubu = python_install_recipe("ubuntu:22.04")
        assert deb == ubu


# ---------------------------------------------------------------------------
# tcl_cli_download_url
# ---------------------------------------------------------------------------


class TestTclCliDownloadUrl:
    def test_default_version(self) -> None:
        url = tcl_cli_download_url()
        assert "tcl-lsp/releases/download/" in url
        assert ".pyz" in url

    def test_custom_version(self) -> None:
        url = tcl_cli_download_url("2.0.0")
        assert "v2.0.0/tcl-2.0.0.pyz" in url


# ---------------------------------------------------------------------------
# available_recipes
# ---------------------------------------------------------------------------


class TestAvailableRecipes:
    def test_returns_all_families(self) -> None:
        recipes = available_recipes()
        assert "debian" in recipes
        assert "alpine" in recipes
        assert "redhat" in recipes

    def test_each_family_has_versions(self) -> None:
        recipes = available_recipes()
        for family, versions in recipes.items():
            assert len(versions) > 0, f"{family} has no versions"
            for v in versions:
                assert v in SUPPORTED_TCL_VERSIONS


# ---------------------------------------------------------------------------
# DockerfileSpec defaults
# ---------------------------------------------------------------------------


class TestDockerfileSpec:
    def test_defaults(self) -> None:
        spec = DockerfileSpec()
        assert spec.base_image == DEFAULT_BASE_IMAGE
        assert spec.tcl_version == DEFAULT_TCL_VERSION
        assert spec.workdir == "/app"
        assert spec.copy_project is True
        assert spec.install_packages is True
        assert spec.create_venv is False
        assert spec.entrypoint is None
        assert spec.extra_packages == []
        assert spec.labels == {}
        assert spec.env == {}
        assert spec.cli_version is None


# ---------------------------------------------------------------------------
# generate_dockerfile
# ---------------------------------------------------------------------------


class TestGenerateDockerfile:
    def test_basic_generation(self) -> None:
        spec = DockerfileSpec()
        result = generate_dockerfile(spec)
        assert result.startswith("FROM debian:bookworm-slim")
        assert "Install Tcl 8.6" in result
        assert "WORKDIR /app" in result
        assert "COPY . ." in result
        assert 'CMD ["tclsh"]' in result

    def test_installs_python3_when_packages_enabled(self) -> None:
        spec = DockerfileSpec()
        result = generate_dockerfile(spec)
        assert "python3" in result
        assert "Install Python 3" in result

    def test_downloads_tcl_cli_pyz(self) -> None:
        spec = DockerfileSpec()
        result = generate_dockerfile(spec)
        assert "tcl.pyz" in result
        assert "tcl-lsp/releases/download/" in result

    def test_pkg_sync_uses_pyz(self) -> None:
        spec = DockerfileSpec()
        result = generate_dockerfile(spec)
        assert "python3 /usr/local/bin/tcl.pyz pkg sync --frozen" in result

    def test_no_python_when_no_packages_no_venv(self) -> None:
        spec = DockerfileSpec(install_packages=False, create_venv=False)
        result = generate_dockerfile(spec)
        assert "Install Python 3" not in result
        assert "tcl.pyz" not in result

    def test_custom_base_image(self) -> None:
        spec = DockerfileSpec(base_image="alpine:3.19")
        result = generate_dockerfile(spec)
        assert result.startswith("FROM alpine:3.19")
        assert "apk" in result

    def test_tcl_90(self) -> None:
        spec = DockerfileSpec(tcl_version="9.0")
        result = generate_dockerfile(spec)
        assert "9.0" in result
        assert "configure" in result

    def test_custom_entrypoint(self) -> None:
        spec = DockerfileSpec(entrypoint="main.tcl")
        result = generate_dockerfile(spec)
        assert 'CMD ["tclsh", "main.tcl"]' in result

    def test_venv_creation_uses_tcl_cli(self) -> None:
        spec = DockerfileSpec(create_venv=True)
        result = generate_dockerfile(spec)
        assert "python3 /usr/local/bin/tcl.pyz venv create .venv" in result
        assert "TCLLIBPATH" in result
        assert ".venv" in result
        assert "TCL_VENV=" in result

    def test_venv_created_before_pkg_sync(self) -> None:
        """When both venv and packages are enabled, venv must come first
        so that ``pkg sync`` materialises packages into ``.venv/lib/``."""
        spec = DockerfileSpec(create_venv=True, install_packages=True)
        result = generate_dockerfile(spec)
        venv_pos = result.index("venv create")
        pkg_pos = result.index("pkg sync")
        assert venv_pos < pkg_pos, "venv create must appear before pkg sync"
        assert "Install Tcl packages into the virtual environment" in result

    def test_venv_paths_follow_workdir(self) -> None:
        """Venv ENV paths must use the configured workdir, not hard-coded /app."""
        spec = DockerfileSpec(create_venv=True, workdir="/opt/myapp")
        result = generate_dockerfile(spec)
        assert 'TCLLIBPATH="/opt/myapp/.venv/lib"' in result
        assert 'PATH="/opt/myapp/.venv/bin:$PATH"' in result
        assert 'TCL_VENV="/opt/myapp/.venv"' in result
        # Must NOT contain /app/.venv when workdir differs.
        assert "/app/.venv" not in result

    def test_venv_paths_default_workdir(self) -> None:
        """With default workdir (/app), venv paths should use /app."""
        spec = DockerfileSpec(create_venv=True)
        result = generate_dockerfile(spec)
        assert 'TCLLIBPATH="/app/.venv/lib"' in result
        assert 'TCL_VENV="/app/.venv"' in result

    def test_no_copy(self) -> None:
        spec = DockerfileSpec(copy_project=False)
        result = generate_dockerfile(spec)
        assert "COPY . ." not in result

    def test_no_packages(self) -> None:
        spec = DockerfileSpec(install_packages=False)
        result = generate_dockerfile(spec)
        assert "pkg sync" not in result

    def test_labels(self) -> None:
        spec = DockerfileSpec(labels={"maintainer": "test", "version": "1.0"})
        result = generate_dockerfile(spec)
        assert 'LABEL maintainer="test"' in result
        assert 'LABEL version="1.0"' in result

    def test_env(self) -> None:
        spec = DockerfileSpec(env={"MY_VAR": "hello"})
        result = generate_dockerfile(spec)
        assert 'ENV MY_VAR="hello"' in result

    def test_extra_packages_debian(self) -> None:
        spec = DockerfileSpec(extra_packages=["curl", "git"])
        result = generate_dockerfile(spec)
        assert "curl" in result
        assert "git" in result
        assert "apt-get" in result

    def test_extra_packages_alpine(self) -> None:
        spec = DockerfileSpec(base_image="alpine:3.19", extra_packages=["curl"])
        result = generate_dockerfile(spec)
        assert "apk add" in result
        assert "curl" in result

    def test_extra_packages_redhat(self) -> None:
        spec = DockerfileSpec(base_image="fedora:39", extra_packages=["curl"])
        result = generate_dockerfile(spec)
        assert "dnf install" in result
        assert "curl" in result

    def test_custom_workdir(self) -> None:
        spec = DockerfileSpec(workdir="/opt/myapp")
        result = generate_dockerfile(spec)
        assert "WORKDIR /opt/myapp" in result

    def test_unsupported_version_raises(self) -> None:
        spec = DockerfileSpec(tcl_version="7.0")
        with pytest.raises(DockerError):
            generate_dockerfile(spec)

    def test_custom_cli_version(self) -> None:
        spec = DockerfileSpec(cli_version="2.0.0")
        result = generate_dockerfile(spec)
        assert "v2.0.0/tcl-2.0.0.pyz" in result

    def test_alpine_with_packages_installs_python(self) -> None:
        spec = DockerfileSpec(base_image="alpine:3.19")
        result = generate_dockerfile(spec)
        assert "apk add --no-cache python3" in result

    def test_redhat_with_venv_installs_python(self) -> None:
        spec = DockerfileSpec(base_image="fedora:39", create_venv=True, install_packages=False)
        result = generate_dockerfile(spec)
        assert "dnf install -y python3" in result


# ---------------------------------------------------------------------------
# write_dockerfile
# ---------------------------------------------------------------------------


class TestWriteDockerfile:
    def test_writes_file(self, tmp_path: Path) -> None:
        spec = DockerfileSpec()
        output = tmp_path / "Dockerfile"
        result = write_dockerfile(output, spec)
        assert result == output
        assert output.exists()
        content = output.read_text()
        assert "FROM debian:bookworm-slim" in content

    def test_refuses_overwrite_without_force(self, tmp_path: Path) -> None:
        output = tmp_path / "Dockerfile"
        output.write_text("existing content")
        spec = DockerfileSpec()
        with pytest.raises(DockerError, match="already exists"):
            write_dockerfile(output, spec)

    def test_overwrites_with_force(self, tmp_path: Path) -> None:
        output = tmp_path / "Dockerfile"
        output.write_text("existing content")
        spec = DockerfileSpec()
        result = write_dockerfile(output, spec, overwrite=True)
        assert result == output
        content = output.read_text()
        assert "FROM debian:bookworm-slim" in content


# ---------------------------------------------------------------------------
# CLI integration (tcl docker create)
# ---------------------------------------------------------------------------


class TestDockerCLI:
    """Integration tests for the ``tcl docker`` CLI verb."""

    def test_docker_create_basic(self, tmp_path: Path) -> None:
        from tooling.tcl.main import main

        output = tmp_path / "Dockerfile"
        exit_code = main(
            [
                "docker",
                "create",
                "debian:bookworm-slim",
                "--output",
                str(output),
            ]
        )
        assert exit_code == 0
        assert output.exists()
        content = output.read_text()
        assert "FROM debian:bookworm-slim" in content
        assert "python3" in content
        assert "tcl.pyz" in content
        assert "pkg sync" in content

    def test_docker_create_alpine_90(self, tmp_path: Path) -> None:
        from tooling.tcl.main import main

        output = tmp_path / "Dockerfile"
        exit_code = main(
            [
                "docker",
                "create",
                "alpine:3.19",
                "--tcl-version",
                "9.0",
                "--output",
                str(output),
            ]
        )
        assert exit_code == 0
        content = output.read_text()
        assert "FROM alpine:3.19" in content
        assert "9.0" in content

    def test_docker_create_with_entrypoint(self, tmp_path: Path) -> None:
        from tooling.tcl.main import main

        output = tmp_path / "Dockerfile"
        exit_code = main(
            [
                "docker",
                "create",
                "ubuntu:22.04",
                "--entrypoint",
                "main.tcl",
                "--output",
                str(output),
            ]
        )
        assert exit_code == 0
        content = output.read_text()
        assert 'CMD ["tclsh", "main.tcl"]' in content

    def test_docker_create_with_venv(self, tmp_path: Path) -> None:
        from tooling.tcl.main import main

        output = tmp_path / "Dockerfile"
        exit_code = main(
            [
                "docker",
                "create",
                "debian:bookworm-slim",
                "--venv",
                "--output",
                str(output),
            ]
        )
        assert exit_code == 0
        content = output.read_text()
        assert "TCLLIBPATH" in content
        assert "tcl.pyz venv create" in content

    def test_docker_create_no_packages(self, tmp_path: Path) -> None:
        from tooling.tcl.main import main

        output = tmp_path / "Dockerfile"
        exit_code = main(
            [
                "docker",
                "create",
                "debian:bookworm-slim",
                "--no-packages",
                "--output",
                str(output),
            ]
        )
        assert exit_code == 0
        content = output.read_text()
        assert "pkg sync" not in content

    def test_docker_create_refuses_overwrite(self, tmp_path: Path) -> None:
        from tooling.tcl.main import main

        output = tmp_path / "Dockerfile"
        output.write_text("existing")
        exit_code = main(
            [
                "docker",
                "create",
                "debian:bookworm-slim",
                "--output",
                str(output),
            ]
        )
        assert exit_code == 1

    def test_docker_create_force_overwrite(self, tmp_path: Path) -> None:
        from tooling.tcl.main import main

        output = tmp_path / "Dockerfile"
        output.write_text("existing")
        exit_code = main(
            [
                "docker",
                "create",
                "debian:bookworm-slim",
                "--output",
                str(output),
                "--force",
            ]
        )
        assert exit_code == 0

    def test_docker_create_json(self, tmp_path: Path) -> None:
        import json
        import sys
        from io import StringIO

        from tooling.tcl.main import main

        output = tmp_path / "Dockerfile"
        captured = StringIO()
        old_stdout = sys.stdout
        sys.stdout = captured
        try:
            exit_code = main(
                [
                    "docker",
                    "create",
                    "debian:bookworm-slim",
                    "--output",
                    str(output),
                    "--json",
                ]
            )
        finally:
            sys.stdout = old_stdout

        assert exit_code == 0
        data = json.loads(captured.getvalue())
        assert data["base_image"] == "debian:bookworm-slim"
        assert data["tcl_version"] == "8.6"
        assert "content" in data

    def test_docker_create_cli_version(self, tmp_path: Path) -> None:
        from tooling.tcl.main import main

        output = tmp_path / "Dockerfile"
        exit_code = main(
            [
                "docker",
                "create",
                "debian:bookworm-slim",
                "--cli-version",
                "2.0.0",
                "--output",
                str(output),
            ]
        )
        assert exit_code == 0
        content = output.read_text()
        assert "v2.0.0/tcl-2.0.0.pyz" in content

    def test_docker_recipe(self) -> None:
        import sys
        from io import StringIO

        from tooling.tcl.main import main

        captured = StringIO()
        old_stdout = sys.stdout
        sys.stdout = captured
        try:
            exit_code = main(["docker", "recipe", "alpine:3.19", "--tcl-version", "8.6"])
        finally:
            sys.stdout = old_stdout

        assert exit_code == 0
        assert "apk" in captured.getvalue()

    def test_docker_info(self) -> None:
        import sys
        from io import StringIO

        from tooling.tcl.main import main

        captured = StringIO()
        old_stdout = sys.stdout
        sys.stdout = captured
        try:
            exit_code = main(["docker", "info"])
        finally:
            sys.stdout = old_stdout

        assert exit_code == 0
        output = captured.getvalue()
        assert "8.6" in output

    def test_docker_info_json(self) -> None:
        import json
        import sys
        from io import StringIO

        from tooling.tcl.main import main

        captured = StringIO()
        old_stdout = sys.stdout
        sys.stdout = captured
        try:
            exit_code = main(["docker", "info", "--json"])
        finally:
            sys.stdout = old_stdout

        assert exit_code == 0
        data = json.loads(captured.getvalue())
        assert "supported_tcl_versions" in data
        assert "families" in data
        assert "8.6" in data["supported_tcl_versions"]
