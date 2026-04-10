"""Dockerfile generation for Tcl projects.

Generates production-ready Dockerfiles that install a specific Tcl
version, optionally bundle project packages via ``tclpkg``, and create
a virtual environment inside the container.

The module provides both a library API (``generate_dockerfile``) for
programmatic use and recipe lookup (``tcl_install_recipe``) for AI
skills that need to compose custom Dockerfiles.
"""

from __future__ import annotations

import textwrap
from dataclasses import dataclass, field
from pathlib import Path

# ---------------------------------------------------------------------------
# Tcl install recipes per base-image family
# ---------------------------------------------------------------------------

# Each recipe is a mapping of base-image prefix → shell commands to install
# a given Tcl version.  The ``{version}`` and ``{version_short}`` placeholders
# are expanded at generation time (e.g. ``8.6`` → ``8.6.16``, ``86``).

_RECIPES: dict[str, dict[str, str]] = {
    # ── Debian / Ubuntu ────────────────────────────────────────────────
    "debian": {
        "8.4": textwrap.dedent("""\
            RUN apt-get update && apt-get install -y --no-install-recommends \\
                    build-essential curl ca-certificates && \\
                curl -fSL "https://prdownloads.sourceforge.net/tcl/tcl8.4.20-src.tar.gz" \\
                    -o /tmp/tcl.tar.gz && \\
                tar -xzf /tmp/tcl.tar.gz -C /tmp && \\
                cd /tmp/tcl8.4.20/unix && \\
                ./configure --prefix=/usr/local && make -j"$(nproc)" && make install && \\
                ln -sf /usr/local/bin/tclsh8.4 /usr/local/bin/tclsh && \\
                rm -rf /tmp/tcl* && \\
                apt-get purge -y --auto-remove build-essential && \\
                rm -rf /var/lib/apt/lists/*"""),
        "8.5": textwrap.dedent("""\
            RUN apt-get update && apt-get install -y --no-install-recommends \\
                    build-essential curl ca-certificates && \\
                curl -fSL "https://prdownloads.sourceforge.net/tcl/tcl8.5.19-src.tar.gz" \\
                    -o /tmp/tcl.tar.gz && \\
                tar -xzf /tmp/tcl.tar.gz -C /tmp && \\
                cd /tmp/tcl8.5.19/unix && \\
                ./configure --prefix=/usr/local && make -j"$(nproc)" && make install && \\
                ln -sf /usr/local/bin/tclsh8.5 /usr/local/bin/tclsh && \\
                rm -rf /tmp/tcl* && \\
                apt-get purge -y --auto-remove build-essential && \\
                rm -rf /var/lib/apt/lists/*"""),
        "8.6": textwrap.dedent("""\
            RUN apt-get update && apt-get install -y --no-install-recommends \\
                    tcl8.6 && \\
                ln -sf /usr/bin/tclsh8.6 /usr/local/bin/tclsh && \\
                rm -rf /var/lib/apt/lists/*"""),
        "9.0": textwrap.dedent("""\
            RUN apt-get update && apt-get install -y --no-install-recommends \\
                    build-essential curl ca-certificates && \\
                curl -fSL "https://prdownloads.sourceforge.net/tcl/tcl9.0.1-src.tar.gz" \\
                    -o /tmp/tcl.tar.gz && \\
                tar -xzf /tmp/tcl.tar.gz -C /tmp && \\
                cd /tmp/tcl9.0.1/unix && \\
                ./configure --prefix=/usr/local && make -j"$(nproc)" && make install && \\
                ln -sf /usr/local/bin/tclsh9.0 /usr/local/bin/tclsh && \\
                rm -rf /tmp/tcl* && \\
                apt-get purge -y --auto-remove build-essential && \\
                rm -rf /var/lib/apt/lists/*"""),
    },
    # ── Alpine ─────────────────────────────────────────────────────────
    "alpine": {
        "8.4": textwrap.dedent("""\
            RUN apk add --no-cache build-base curl && \\
                curl -fSL "https://prdownloads.sourceforge.net/tcl/tcl8.4.20-src.tar.gz" \\
                    -o /tmp/tcl.tar.gz && \\
                tar -xzf /tmp/tcl.tar.gz -C /tmp && \\
                cd /tmp/tcl8.4.20/unix && \\
                ./configure --prefix=/usr/local && make -j"$(nproc)" && make install && \\
                ln -sf /usr/local/bin/tclsh8.4 /usr/local/bin/tclsh && \\
                rm -rf /tmp/tcl* && \\
                apk del build-base"""),
        "8.5": textwrap.dedent("""\
            RUN apk add --no-cache build-base curl && \\
                curl -fSL "https://prdownloads.sourceforge.net/tcl/tcl8.5.19-src.tar.gz" \\
                    -o /tmp/tcl.tar.gz && \\
                tar -xzf /tmp/tcl.tar.gz -C /tmp && \\
                cd /tmp/tcl8.5.19/unix && \\
                ./configure --prefix=/usr/local && make -j"$(nproc)" && make install && \\
                ln -sf /usr/local/bin/tclsh8.5 /usr/local/bin/tclsh && \\
                rm -rf /tmp/tcl* && \\
                apk del build-base"""),
        "8.6": textwrap.dedent("""\
            RUN apk add --no-cache tcl"""),
        "9.0": textwrap.dedent("""\
            RUN apk add --no-cache build-base curl && \\
                curl -fSL "https://prdownloads.sourceforge.net/tcl/tcl9.0.1-src.tar.gz" \\
                    -o /tmp/tcl.tar.gz && \\
                tar -xzf /tmp/tcl.tar.gz -C /tmp && \\
                cd /tmp/tcl9.0.1/unix && \\
                ./configure --prefix=/usr/local && make -j"$(nproc)" && make install && \\
                ln -sf /usr/local/bin/tclsh9.0 /usr/local/bin/tclsh && \\
                rm -rf /tmp/tcl* && \\
                apk del build-base"""),
    },
    # ── RHEL / Fedora / CentOS ─────────────────────────────────────────
    "redhat": {
        "8.4": textwrap.dedent("""\
            RUN dnf install -y gcc make curl && \\
                curl -fSL "https://prdownloads.sourceforge.net/tcl/tcl8.4.20-src.tar.gz" \\
                    -o /tmp/tcl.tar.gz && \\
                tar -xzf /tmp/tcl.tar.gz -C /tmp && \\
                cd /tmp/tcl8.4.20/unix && \\
                ./configure --prefix=/usr/local && make -j"$(nproc)" && make install && \\
                ln -sf /usr/local/bin/tclsh8.4 /usr/local/bin/tclsh && \\
                rm -rf /tmp/tcl* && \\
                dnf remove -y gcc make && dnf clean all"""),
        "8.5": textwrap.dedent("""\
            RUN dnf install -y gcc make curl && \\
                curl -fSL "https://prdownloads.sourceforge.net/tcl/tcl8.5.19-src.tar.gz" \\
                    -o /tmp/tcl.tar.gz && \\
                tar -xzf /tmp/tcl.tar.gz -C /tmp && \\
                cd /tmp/tcl8.5.19/unix && \\
                ./configure --prefix=/usr/local && make -j"$(nproc)" && make install && \\
                ln -sf /usr/local/bin/tclsh8.5 /usr/local/bin/tclsh && \\
                rm -rf /tmp/tcl* && \\
                dnf remove -y gcc make && dnf clean all"""),
        "8.6": textwrap.dedent("""\
            RUN dnf install -y tcl && dnf clean all"""),
        "9.0": textwrap.dedent("""\
            RUN dnf install -y gcc make curl && \\
                curl -fSL "https://prdownloads.sourceforge.net/tcl/tcl9.0.1-src.tar.gz" \\
                    -o /tmp/tcl.tar.gz && \\
                tar -xzf /tmp/tcl.tar.gz -C /tmp && \\
                cd /tmp/tcl9.0.1/unix && \\
                ./configure --prefix=/usr/local && make -j"$(nproc)" && make install && \\
                ln -sf /usr/local/bin/tclsh9.0 /usr/local/bin/tclsh && \\
                rm -rf /tmp/tcl* && \\
                dnf remove -y gcc make && dnf clean all"""),
    },
}

# Map well-known image prefixes to recipe families.
_IMAGE_FAMILY: list[tuple[str, str]] = [
    ("alpine", "alpine"),
    ("debian", "debian"),
    ("ubuntu", "debian"),
    ("buildpack-deps", "debian"),
    ("slim", "debian"),
    ("fedora", "redhat"),
    ("centos", "redhat"),
    ("rockylinux", "redhat"),
    ("almalinux", "redhat"),
    ("amazonlinux", "redhat"),
    ("oraclelinux", "redhat"),
    ("rhel", "redhat"),
    ("redhat", "redhat"),
]

SUPPORTED_TCL_VERSIONS = ("8.4", "8.5", "8.6", "9.0")

DEFAULT_TCL_VERSION = "8.6"

DEFAULT_BASE_IMAGE = "debian:bookworm-slim"


class DockerError(ValueError):
    """Raised when Dockerfile generation fails."""


# ---------------------------------------------------------------------------
# Public helpers
# ---------------------------------------------------------------------------


def detect_image_family(image: str) -> str:
    """Return the recipe family key for the given Docker image name.

    >>> detect_image_family("alpine:3.19")
    'alpine'
    >>> detect_image_family("ubuntu:22.04")
    'debian'
    >>> detect_image_family("fedora:39")
    'redhat'
    >>> detect_image_family("custom-image:latest")
    'debian'
    """
    lower = image.lower()
    for prefix, family in _IMAGE_FAMILY:
        if lower.startswith(prefix):
            return family
    # Default to debian for unknown images.
    return "debian"


def tcl_install_recipe(image: str, tcl_version: str) -> str:
    """Return the Dockerfile snippet that installs Tcl *tcl_version* on *image*.

    Raises ``DockerError`` if the requested version is unsupported.
    """
    if tcl_version not in SUPPORTED_TCL_VERSIONS:
        raise DockerError(
            f"unsupported Tcl version: {tcl_version} "
            f"(supported: {', '.join(SUPPORTED_TCL_VERSIONS)})"
        )
    family = detect_image_family(image)
    recipes = _RECIPES.get(family)
    if recipes is None:
        raise DockerError(f"no install recipe for image family: {family}")
    recipe = recipes.get(tcl_version)
    if recipe is None:
        raise DockerError(
            f"no recipe for Tcl {tcl_version} on {family} "
            f"(available: {', '.join(sorted(recipes))})"
        )
    return recipe


def available_recipes() -> dict[str, list[str]]:
    """Return ``{family: [versions]}`` for all known recipe families."""
    return {family: sorted(recipes) for family, recipes in sorted(_RECIPES.items())}


# ---------------------------------------------------------------------------
# Dockerfile generation
# ---------------------------------------------------------------------------


@dataclass
class DockerfileSpec:
    """Parameters for Dockerfile generation."""

    base_image: str = DEFAULT_BASE_IMAGE
    tcl_version: str = DEFAULT_TCL_VERSION
    workdir: str = "/app"
    copy_project: bool = True
    install_packages: bool = True
    create_venv: bool = False
    entrypoint: str | None = None
    extra_packages: list[str] = field(default_factory=list)
    labels: dict[str, str] = field(default_factory=dict)
    env: dict[str, str] = field(default_factory=dict)


def generate_dockerfile(spec: DockerfileSpec) -> str:
    """Generate a complete Dockerfile from a ``DockerfileSpec``.

    Returns the Dockerfile content as a string.
    """
    lines: list[str] = []

    # ── FROM ───────────────────────────────────────────────────────────
    lines.append(f"FROM {spec.base_image}")
    lines.append("")

    # ── Labels ─────────────────────────────────────────────────────────
    if spec.labels:
        for key, value in sorted(spec.labels.items()):
            lines.append(f'LABEL {key}="{value}"')
        lines.append("")

    # ── Environment ────────────────────────────────────────────────────
    if spec.env:
        for key, value in sorted(spec.env.items()):
            lines.append(f'ENV {key}="{value}"')
        lines.append("")

    # ── Tcl install ────────────────────────────────────────────────────
    lines.append(f"# Install Tcl {spec.tcl_version}")
    lines.append(tcl_install_recipe(spec.base_image, spec.tcl_version))
    lines.append("")

    # ── Extra packages ─────────────────────────────────────────────────
    if spec.extra_packages:
        family = detect_image_family(spec.base_image)
        pkg_list = " \\\n                    ".join(spec.extra_packages)
        if family == "alpine":
            lines.append(f"RUN apk add --no-cache {pkg_list}")
        elif family == "redhat":
            lines.append(f"RUN dnf install -y {pkg_list} && dnf clean all")
        else:
            lines.append(
                f"RUN apt-get update && apt-get install -y --no-install-recommends \\\n"
                f"                    {pkg_list} && \\\n"
                f"                rm -rf /var/lib/apt/lists/*"
            )
        lines.append("")

    # ── Workdir ────────────────────────────────────────────────────────
    lines.append(f"WORKDIR {spec.workdir}")
    lines.append("")

    # ── Copy project ───────────────────────────────────────────────────
    if spec.copy_project:
        lines.append("COPY . .")
        lines.append("")

    # ── tclpkg install ─────────────────────────────────────────────────
    if spec.install_packages:
        lines.append("# Install Tcl packages (if tclpkg.tcl manifest exists)")
        lines.append(
            "RUN if [ -f tclpkg.tcl ]; then "
            "tclsh /app/lib/tclpkg/main.tcl pkg sync --frozen 2>/dev/null || true; fi"
        )
        lines.append("")

    # ── venv ───────────────────────────────────────────────────────────
    if spec.create_venv:
        lines.append("# Create Tcl virtual environment")
        lines.append("RUN mkdir -p .venv/bin .venv/lib && \\")
        tclsh_name = f"tclsh{spec.tcl_version}" if spec.tcl_version != "8.6" else "tclsh8.6"
        lines.append(
            f'    printf \'#!/bin/sh\\nexec {tclsh_name} "$@"\\n\' > .venv/bin/tclsh && \\'
        )
        lines.append("    chmod +x .venv/bin/tclsh")
        lines.append('ENV TCLLIBPATH="/app/.venv/lib"')
        lines.append('ENV PATH="/app/.venv/bin:$PATH"')
        lines.append("")

    # ── Entrypoint ─────────────────────────────────────────────────────
    if spec.entrypoint:
        lines.append(f'CMD ["tclsh", "{spec.entrypoint}"]')
    else:
        lines.append('CMD ["tclsh"]')
    lines.append("")

    return "\n".join(lines)


def write_dockerfile(
    output: Path,
    spec: DockerfileSpec,
    *,
    overwrite: bool = False,
) -> Path:
    """Generate and write a Dockerfile to *output*.

    Returns the path written to.  Raises ``DockerError`` if the file
    exists and *overwrite* is ``False``.
    """
    if output.exists() and not overwrite:
        raise DockerError(f"file already exists: {output} (use --force to overwrite)")
    content = generate_dockerfile(spec)
    output.write_text(content, encoding="utf-8")
    return output


__all__ = [
    "DEFAULT_BASE_IMAGE",
    "DEFAULT_TCL_VERSION",
    "DockerError",
    "DockerfileSpec",
    "SUPPORTED_TCL_VERSIONS",
    "available_recipes",
    "detect_image_family",
    "generate_dockerfile",
    "tcl_install_recipe",
    "write_dockerfile",
]
