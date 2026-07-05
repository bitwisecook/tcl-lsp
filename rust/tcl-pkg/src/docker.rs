// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Dockerfile generation for Tcl projects.
//!
//! Generates production-ready
//! Dockerfiles that install a specific Tcl version, download the `tcl` CLI
//! zipapp, and wire `tcl pkg sync` / `tcl venv create`. Also exposes recipe
//! lookup helpers for AI skills composing custom Dockerfiles.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::errors::TclPkgError;

pub const SUPPORTED_TCL_VERSIONS: [&str; 4] = ["8.4", "8.5", "8.6", "9.0"];
pub const DEFAULT_TCL_VERSION: &str = "8.6";
pub const DEFAULT_BASE_IMAGE: &str = "debian:bookworm-slim";

const TCL_CLI_URL_PREFIX: &str = "https://github.com/bitwisecook/tcl-lsp/releases/download/v";
const DEFAULT_CLI_VERSION: &str = "1.6.0";

// Map well-known image prefixes to recipe families.
const IMAGE_FAMILY: &[(&str, &str)] = &[
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
];

fn docker_error(message: impl Into<String>) -> TclPkgError {
    TclPkgError::new(message)
}

fn recipe(family: &str, version: &str) -> Option<&'static str> {
    let table: &[(&str, &str)] = match family {
        "debian" => DEBIAN_RECIPES,
        "alpine" => ALPINE_RECIPES,
        "redhat" => REDHAT_RECIPES,
        _ => return None,
    };
    table.iter().find(|(v, _)| *v == version).map(|(_, r)| *r)
}

fn family_versions(family: &str) -> Option<&'static [(&'static str, &'static str)]> {
    match family {
        "debian" => Some(DEBIAN_RECIPES),
        "alpine" => Some(ALPINE_RECIPES),
        "redhat" => Some(REDHAT_RECIPES),
        _ => None,
    }
}

const PYTHON_INSTALL_DEBIAN: &str = "RUN apt-get update && apt-get install -y --no-install-recommends \\\n        python3 curl ca-certificates && \\\n    rm -rf /var/lib/apt/lists/*";
const PYTHON_INSTALL_ALPINE: &str = "RUN apk add --no-cache python3 curl";
const PYTHON_INSTALL_REDHAT: &str = "RUN dnf install -y python3 curl && dnf clean all";

fn strip_registry(image: &str) -> String {
    let parts: Vec<&str> = image.split('/').collect();
    if parts.len() >= 2 && (parts[0].contains('.') || parts[0].contains(':')) {
        return (*parts.last().unwrap()).to_string();
    }
    if parts.len() == 3 {
        return (*parts.last().unwrap()).to_string();
    }
    image.to_string()
}

/// Return the recipe family key for the given Docker image name.
#[must_use]
pub fn detect_image_family(image: &str) -> String {
    let lower = strip_registry(image).to_lowercase();
    for (prefix, family) in IMAGE_FAMILY {
        if lower.starts_with(prefix) {
            return (*family).to_string();
        }
    }
    "debian".to_string()
}

/// Return the Dockerfile snippet that installs Tcl `tcl_version` on `image`.
pub fn tcl_install_recipe(image: &str, tcl_version: &str) -> Result<String, TclPkgError> {
    if !SUPPORTED_TCL_VERSIONS.contains(&tcl_version) {
        return Err(docker_error(format!(
            "unsupported Tcl version: {tcl_version} (supported: {})",
            SUPPORTED_TCL_VERSIONS.join(", ")
        )));
    }
    let family = detect_image_family(image);
    let Some(versions) = family_versions(&family) else {
        return Err(docker_error(format!(
            "no install recipe for image family: {family}"
        )));
    };
    recipe(&family, tcl_version)
        .map(ToString::to_string)
        .ok_or_else(|| {
            let mut available: Vec<&str> = versions.iter().map(|(v, _)| *v).collect();
            available.sort_unstable();
            docker_error(format!(
                "no recipe for Tcl {tcl_version} on {family} (available: {})",
                available.join(", ")
            ))
        })
}

/// Return the Dockerfile snippet that installs Python 3 on `image`.
pub fn python_install_recipe(image: &str) -> Result<String, TclPkgError> {
    match detect_image_family(image).as_str() {
        "debian" => Ok(PYTHON_INSTALL_DEBIAN.to_string()),
        "alpine" => Ok(PYTHON_INSTALL_ALPINE.to_string()),
        "redhat" => Ok(PYTHON_INSTALL_REDHAT.to_string()),
        family => Err(docker_error(format!(
            "no Python install recipe for image family: {family}"
        ))),
    }
}

/// Return the GitHub release URL for the `tcl` CLI zipapp.
#[must_use]
pub fn tcl_cli_download_url(cli_version: Option<&str>) -> String {
    let version = cli_version.unwrap_or(DEFAULT_CLI_VERSION);
    format!("{TCL_CLI_URL_PREFIX}{version}/tcl-{version}.pyz")
}

/// Return `{family: [versions]}` for all known recipe families (sorted).
#[must_use]
pub fn available_recipes() -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    for family in ["alpine", "debian", "redhat"] {
        if let Some(versions) = family_versions(family) {
            let mut vs: Vec<String> = versions.iter().map(|(v, _)| (*v).to_string()).collect();
            vs.sort();
            out.insert(family.to_string(), vs);
        }
    }
    out
}

/// Parameters for Dockerfile generation.
#[derive(Debug, Clone)]
pub struct DockerfileSpec {
    pub base_image: String,
    pub tcl_version: String,
    pub workdir: String,
    pub copy_project: bool,
    pub install_packages: bool,
    pub create_venv: bool,
    pub entrypoint: Option<String>,
    pub extra_packages: Vec<String>,
    pub labels: BTreeMap<String, String>,
    pub env: BTreeMap<String, String>,
    pub cli_version: Option<String>,
}

impl Default for DockerfileSpec {
    fn default() -> Self {
        Self {
            base_image: DEFAULT_BASE_IMAGE.to_string(),
            tcl_version: DEFAULT_TCL_VERSION.to_string(),
            workdir: "/app".to_string(),
            copy_project: true,
            install_packages: true,
            create_venv: false,
            entrypoint: None,
            extra_packages: Vec::new(),
            labels: BTreeMap::new(),
            env: BTreeMap::new(),
            cli_version: None,
        }
    }
}

/// Generate a complete Dockerfile from a [`DockerfileSpec`].
pub fn generate_dockerfile(spec: &DockerfileSpec) -> Result<String, TclPkgError> {
    let mut lines: Vec<String> = Vec::new();
    let family = detect_image_family(&spec.base_image);
    let needs_cli = spec.install_packages || spec.create_venv;

    lines.push(format!("FROM {}", spec.base_image));
    lines.push(String::new());

    if !spec.labels.is_empty() {
        for (key, value) in &spec.labels {
            lines.push(format!("LABEL {key}=\"{value}\""));
        }
        lines.push(String::new());
    }

    if !spec.env.is_empty() {
        for (key, value) in &spec.env {
            lines.push(format!("ENV {key}=\"{value}\""));
        }
        lines.push(String::new());
    }

    lines.push(format!("# Install Tcl {}", spec.tcl_version));
    lines.push(tcl_install_recipe(&spec.base_image, &spec.tcl_version)?);
    lines.push(String::new());

    if needs_cli {
        lines.push("# Install Python 3 (required by the tcl CLI tool)".to_string());
        lines.push(python_install_recipe(&spec.base_image)?);
        lines.push(String::new());

        let url = tcl_cli_download_url(spec.cli_version.as_deref());
        lines.push("# Download the tcl CLI zipapp from GitHub releases".to_string());
        lines.push(format!(
            "RUN curl -fSL \"{url}\" \\\n        -o /usr/local/bin/tcl.pyz && \\\n    chmod +x /usr/local/bin/tcl.pyz"
        ));
        lines.push(String::new());
    }

    if !spec.extra_packages.is_empty() {
        let pkg_list = spec.extra_packages.join(" \\\n                    ");
        if family == "alpine" {
            lines.push(format!("RUN apk add --no-cache {pkg_list}"));
        } else if family == "redhat" {
            lines.push(format!("RUN dnf install -y {pkg_list} && dnf clean all"));
        } else {
            lines.push(format!(
                "RUN apt-get update && apt-get install -y --no-install-recommends \\\n                    {pkg_list} && \\\n                rm -rf /var/lib/apt/lists/*"
            ));
        }
        lines.push(String::new());
    }

    lines.push(format!("WORKDIR {}", spec.workdir));
    lines.push(String::new());

    if spec.copy_project {
        lines.push("COPY . .".to_string());
        lines.push(String::new());
    }

    if spec.create_venv {
        let venv_abs = format!("{}/.venv", spec.workdir);
        lines.push("# Create Tcl virtual environment".to_string());
        lines.push(format!(
            "RUN python3 /usr/local/bin/tcl.pyz venv create .venv --tcl {}",
            spec.tcl_version
        ));
        lines.push(format!("ENV TCLLIBPATH=\"{venv_abs}/lib\""));
        lines.push(format!("ENV PATH=\"{venv_abs}/bin:$PATH\""));
        lines.push(format!("ENV TCL_VENV=\"{venv_abs}\""));
        lines.push(String::new());
    }

    if spec.install_packages {
        if spec.create_venv {
            lines.push("# Install Tcl packages into the virtual environment".to_string());
        } else {
            lines.push("# Install Tcl packages from lockfile".to_string());
        }
        lines.push(
            "RUN if [ -f tclpkg.lock ]; then python3 /usr/local/bin/tcl.pyz pkg install --frozen; fi"
                .to_string(),
        );
        lines.push(String::new());
    }

    if let Some(entry) = &spec.entrypoint {
        lines.push(format!("CMD [\"tclsh\", \"{entry}\"]"));
    } else {
        lines.push("CMD [\"tclsh\"]".to_string());
    }
    lines.push(String::new());

    Ok(lines.join("\n"))
}

/// Generate and write a Dockerfile to `output`.
pub fn write_dockerfile(
    output: &Path,
    spec: &DockerfileSpec,
    overwrite: bool,
) -> Result<PathBuf, TclPkgError> {
    if output.exists() && !overwrite {
        return Err(docker_error(format!(
            "file already exists: {} (use --force to overwrite)",
            output.display()
        )));
    }
    let content = generate_dockerfile(spec)?;
    std::fs::write(output, content)
        .map_err(|e| docker_error(format!("cannot write {}: {e}", output.display())))?;
    Ok(output.to_path_buf())
}

// Recipe tables.

const DEBIAN_RECIPES: &[(&str, &str)] = &[
    (
        "8.4",
        "RUN apt-get update && apt-get install -y --no-install-recommends \\\n        build-essential curl ca-certificates && \\\n    curl -fSL \"https://prdownloads.sourceforge.net/tcl/tcl8.4.20-src.tar.gz\" \\\n        -o /tmp/tcl.tar.gz && \\\n    tar -xzf /tmp/tcl.tar.gz -C /tmp && \\\n    cd /tmp/tcl8.4.20/unix && \\\n    ./configure --prefix=/usr/local && make -j\"$(nproc)\" && make install && \\\n    ln -sf /usr/local/bin/tclsh8.4 /usr/local/bin/tclsh && \\\n    rm -rf /tmp/tcl* && \\\n    apt-get purge -y --auto-remove build-essential && \\\n    rm -rf /var/lib/apt/lists/*",
    ),
    (
        "8.5",
        "RUN apt-get update && apt-get install -y --no-install-recommends \\\n        build-essential curl ca-certificates && \\\n    curl -fSL \"https://prdownloads.sourceforge.net/tcl/tcl8.5.19-src.tar.gz\" \\\n        -o /tmp/tcl.tar.gz && \\\n    tar -xzf /tmp/tcl.tar.gz -C /tmp && \\\n    cd /tmp/tcl8.5.19/unix && \\\n    ./configure --prefix=/usr/local && make -j\"$(nproc)\" && make install && \\\n    ln -sf /usr/local/bin/tclsh8.5 /usr/local/bin/tclsh && \\\n    rm -rf /tmp/tcl* && \\\n    apt-get purge -y --auto-remove build-essential && \\\n    rm -rf /var/lib/apt/lists/*",
    ),
    (
        "8.6",
        "RUN apt-get update && apt-get install -y --no-install-recommends \\\n        tcl8.6 && \\\n    ln -sf /usr/bin/tclsh8.6 /usr/local/bin/tclsh && \\\n    rm -rf /var/lib/apt/lists/*",
    ),
    (
        "9.0",
        "RUN apt-get update && apt-get install -y --no-install-recommends \\\n        build-essential curl ca-certificates zlib1g-dev && \\\n    curl -fSL \"https://prdownloads.sourceforge.net/tcl/tcl9.0.1-src.tar.gz\" \\\n        -o /tmp/tcl.tar.gz && \\\n    tar -xzf /tmp/tcl.tar.gz -C /tmp && \\\n    cd /tmp/tcl9.0.1/unix && \\\n    ./configure --prefix=/usr/local && make -j\"$(nproc)\" && make install && \\\n    ln -sf /usr/local/bin/tclsh9.0 /usr/local/bin/tclsh && \\\n    rm -rf /tmp/tcl* && \\\n    apt-get purge -y --auto-remove build-essential zlib1g-dev && \\\n    rm -rf /var/lib/apt/lists/*",
    ),
];

const ALPINE_RECIPES: &[(&str, &str)] = &[
    (
        "8.4",
        "RUN apk add --no-cache build-base curl && \\\n    curl -fSL \"https://prdownloads.sourceforge.net/tcl/tcl8.4.20-src.tar.gz\" \\\n        -o /tmp/tcl.tar.gz && \\\n    tar -xzf /tmp/tcl.tar.gz -C /tmp && \\\n    cd /tmp/tcl8.4.20/unix && \\\n    ./configure --prefix=/usr/local && make -j\"$(nproc)\" && make install && \\\n    ln -sf /usr/local/bin/tclsh8.4 /usr/local/bin/tclsh && \\\n    rm -rf /tmp/tcl* && \\\n    apk del build-base",
    ),
    (
        "8.5",
        "RUN apk add --no-cache build-base curl && \\\n    curl -fSL \"https://prdownloads.sourceforge.net/tcl/tcl8.5.19-src.tar.gz\" \\\n        -o /tmp/tcl.tar.gz && \\\n    tar -xzf /tmp/tcl.tar.gz -C /tmp && \\\n    cd /tmp/tcl8.5.19/unix && \\\n    ./configure --prefix=/usr/local && make -j\"$(nproc)\" && make install && \\\n    ln -sf /usr/local/bin/tclsh8.5 /usr/local/bin/tclsh && \\\n    rm -rf /tmp/tcl* && \\\n    apk del build-base",
    ),
    ("8.6", "RUN apk add --no-cache tcl"),
    (
        "9.0",
        "RUN apk add --no-cache build-base curl zlib-dev && \\\n    curl -fSL \"https://prdownloads.sourceforge.net/tcl/tcl9.0.1-src.tar.gz\" \\\n        -o /tmp/tcl.tar.gz && \\\n    tar -xzf /tmp/tcl.tar.gz -C /tmp && \\\n    cd /tmp/tcl9.0.1/unix && \\\n    ./configure --prefix=/usr/local && make -j\"$(nproc)\" && make install && \\\n    ln -sf /usr/local/bin/tclsh9.0 /usr/local/bin/tclsh && \\\n    rm -rf /tmp/tcl* && \\\n    apk del build-base zlib-dev",
    ),
];

const REDHAT_RECIPES: &[(&str, &str)] = &[
    (
        "8.4",
        "RUN dnf install -y gcc make curl && \\\n    curl -fSL \"https://prdownloads.sourceforge.net/tcl/tcl8.4.20-src.tar.gz\" \\\n        -o /tmp/tcl.tar.gz && \\\n    tar -xzf /tmp/tcl.tar.gz -C /tmp && \\\n    cd /tmp/tcl8.4.20/unix && \\\n    ./configure --prefix=/usr/local && make -j\"$(nproc)\" && make install && \\\n    ln -sf /usr/local/bin/tclsh8.4 /usr/local/bin/tclsh && \\\n    rm -rf /tmp/tcl* && \\\n    dnf remove -y gcc make && dnf clean all",
    ),
    (
        "8.5",
        "RUN dnf install -y gcc make curl && \\\n    curl -fSL \"https://prdownloads.sourceforge.net/tcl/tcl8.5.19-src.tar.gz\" \\\n        -o /tmp/tcl.tar.gz && \\\n    tar -xzf /tmp/tcl.tar.gz -C /tmp && \\\n    cd /tmp/tcl8.5.19/unix && \\\n    ./configure --prefix=/usr/local && make -j\"$(nproc)\" && make install && \\\n    ln -sf /usr/local/bin/tclsh8.5 /usr/local/bin/tclsh && \\\n    rm -rf /tmp/tcl* && \\\n    dnf remove -y gcc make && dnf clean all",
    ),
    ("8.6", "RUN dnf install -y tcl && dnf clean all"),
    (
        "9.0",
        "RUN dnf install -y gcc make curl zlib-devel && \\\n    curl -fSL \"https://prdownloads.sourceforge.net/tcl/tcl9.0.1-src.tar.gz\" \\\n        -o /tmp/tcl.tar.gz && \\\n    tar -xzf /tmp/tcl.tar.gz -C /tmp && \\\n    cd /tmp/tcl9.0.1/unix && \\\n    ./configure --prefix=/usr/local && make -j\"$(nproc)\" && make install && \\\n    ln -sf /usr/local/bin/tclsh9.0 /usr/local/bin/tclsh && \\\n    rm -rf /tmp/tcl* && \\\n    dnf remove -y gcc make zlib-devel && dnf clean all",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_detection() {
        assert_eq!(detect_image_family("alpine:3.19"), "alpine");
        assert_eq!(
            detect_image_family("docker.io/library/alpine:3.19"),
            "alpine"
        );
        assert_eq!(detect_image_family("ubuntu:22.04"), "debian");
        assert_eq!(detect_image_family("fedora:39"), "redhat");
        assert_eq!(detect_image_family("quay.io/fedora/fedora:39"), "redhat");
        assert_eq!(detect_image_family("custom-image:latest"), "debian");
    }

    #[test]
    fn recipe_lookup() {
        let r = tcl_install_recipe("alpine:3.19", "8.6").unwrap();
        assert_eq!(r, "RUN apk add --no-cache tcl");
        assert!(tcl_install_recipe("alpine:3.19", "7.0").is_err());
    }

    #[test]
    fn cli_url() {
        assert_eq!(
            tcl_cli_download_url(None),
            "https://github.com/bitwisecook/tcl-lsp/releases/download/v1.6.0/tcl-1.6.0.pyz"
        );
        assert_eq!(
            tcl_cli_download_url(Some("2.0.0")),
            "https://github.com/bitwisecook/tcl-lsp/releases/download/v2.0.0/tcl-2.0.0.pyz"
        );
    }

    #[test]
    fn available() {
        let recipes = available_recipes();
        assert_eq!(recipes["debian"], vec!["8.4", "8.5", "8.6", "9.0"]);
        assert_eq!(
            recipes.keys().collect::<Vec<_>>(),
            vec!["alpine", "debian", "redhat"]
        );
    }

    #[test]
    fn generate_minimal() {
        let spec = DockerfileSpec {
            base_image: "debian:bookworm-slim".to_string(),
            tcl_version: "8.6".to_string(),
            install_packages: false,
            create_venv: false,
            ..Default::default()
        };
        let out = generate_dockerfile(&spec).unwrap();
        assert!(out.starts_with("FROM debian:bookworm-slim\n"));
        assert!(out.contains("# Install Tcl 8.6"));
        assert!(out.contains("WORKDIR /app"));
        assert!(out.contains("COPY . ."));
        assert!(out.trim_end().ends_with("CMD [\"tclsh\"]"));
    }
}
