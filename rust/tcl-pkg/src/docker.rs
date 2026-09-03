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
//! Generates production-ready Dockerfiles that install a specific Tcl version,
//! download the **native** `tcl` CLI binary for the target architecture from a
//! GitHub release, verify it against the release's `SHA256SUMS`, and wire
//! `tcl pkg install --frozen` / `tcl venv create`. Nothing in the generated
//! image needs a Python interpreter. Also exposes recipe lookup helpers for AI
//! skills composing custom Dockerfiles.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::errors::TclPkgError;

pub const SUPPORTED_TCL_VERSIONS: [&str; 4] = ["8.4", "8.5", "8.6", "9.0"];
pub const DEFAULT_TCL_VERSION: &str = "8.6";
pub const DEFAULT_BASE_IMAGE: &str = "debian:bookworm-slim";

const DEFAULT_BASE_IMAGE_COMMENT: [&str; 2] = [
    "# Published tcl-lsp Linux binaries require glibc, so Debian is the safe default.",
    "# If Alpine/musl is required, build tcl-lsp from source inside the image.",
];
const MUSL_BASE_IMAGE_COMMENT: [&str; 2] = [
    "# Published tcl-lsp Linux binaries cannot run here because Alpine uses musl.",
    "# Build tcl-lsp from source inside the image if its tools are required.",
];
const ALTERNATE_GLIBC_BASE_IMAGE_COMMENT: [&str; 2] = [
    "# Published tcl-lsp Linux binaries require a glibc-compatible base image.",
    "# Debian bookworm-slim is the default; Alpine/musl requires a source build.",
];

/// The GitHub repository the release assets are published from.
pub const RELEASE_REPO: &str = "bitwisecook/tcl-lsp";

/// The version `tcl-version` stamps when the checkout has no reachable tag.
/// It names no release, so it can never be pinned in a generated Dockerfile.
const MANIFEST_PLACEHOLDER_VERSION: &str = "0.1.0";

/// Docker's `TARGETARCH` (and the `uname -m` spelling it degrades to under the
/// classic builder) mapped to the Rust target triple the release assets are
/// named for. Keep in sync with the Linux legs of `build-server-matrix` in
/// `.github/workflows/ci.yml`.
pub const RELEASE_TRIPLES: &[(&str, &str, &str)] = &[
    ("amd64", "x86_64", "x86_64-unknown-linux-gnu"),
    ("arm64", "aarch64", "aarch64-unknown-linux-gnu"),
    ("riscv64", "riscv64", "riscv64gc-unknown-linux-gnu"),
];

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

// Fetching and verifying the release asset needs curl, a trust store, and
// sha256sum/awk (already in coreutils everywhere below).
const CLI_PREREQ_DEBIAN: &str = "RUN apt-get update && apt-get install -y --no-install-recommends \\\n        ca-certificates curl && \\\n    rm -rf /var/lib/apt/lists/*";
// `dnf install curl` on an image carrying curl-minimal is a package swap, not
// an install — guard on the binary so a present curl is left alone.
const CLI_PREREQ_REDHAT: &str = "RUN if ! command -v curl >/dev/null 2>&1; then dnf install -y curl; fi && \\\n    dnf install -y ca-certificates && \\\n    dnf clean all";

/// The image families the native `tcl` CLI can be installed on, and what each
/// needs first. One table so [`cli_prereq_recipe`] and [`cli_capable_families`]
/// cannot drift into advertising a family that errors, or omitting one that
/// works. Sorted, because `cli_capable_families` is a user-facing listing.
const CLI_PREREQS: &[(&str, &str)] =
    &[("debian", CLI_PREREQ_DEBIAN), ("redhat", CLI_PREREQ_REDHAT)];

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

/// Return the Dockerfile snippet that installs what fetching and verifying the
/// native `tcl` release asset needs on `image`: curl and a CA bundle.
///
/// Alpine is rejected. Every published Linux `tcl` asset is glibc-linked — the
/// release matrix has no musl leg — and `gcompat` does not close the gap
/// (`fcntl64` and `__res_init` are not among the symbols it re-exports, so the
/// binary dies in the dynamic loader). An Alpine image that needs the CLI must
/// build tcl-lsp from source for musl. A Tcl-only Alpine image is still fine:
/// drop the CLI verbs (`--no-packages`, no `--venv`) and nothing is fetched.
pub fn cli_prereq_recipe(image: &str) -> Result<String, TclPkgError> {
    let family = detect_image_family(image);
    if let Some((_, prereq)) = CLI_PREREQS.iter().find(|(f, _)| *f == family) {
        return Ok((*prereq).to_string());
    }
    if family == "alpine" {
        return Err(docker_error(
            "the native tcl CLI has no musl release asset, so it cannot run on \
             alpine (its glibc shim is missing fcntl64 and __res_init). Use the \
             default Debian image, or if Alpine is required, build tcl-lsp from \
             source for musl inside the image. A Tcl-only Alpine image can still \
             use --no-packages and no --venv.",
        ));
    }
    Err(docker_error(format!(
        "no tcl CLI prerequisite recipe for image family: {family}"
    )))
}

/// The image families a generated Dockerfile can install the native `tcl` CLI
/// on, sorted. Every family in [`available_recipes`] can still install Tcl
/// itself; only these can also carry the CLI.
#[must_use]
pub fn cli_capable_families() -> Vec<String> {
    CLI_PREREQS.iter().map(|(f, _)| (*f).to_string()).collect()
}

/// The release version a generated Dockerfile pins by default.
///
/// Derived from the version stamped into this binary, so a generated image
/// installs the CLI line that generated it. Build metadata (`+g<hash>`,
/// `.dirty`) and the `git describe` distance (`-<n>`) are stripped, leaving the
/// tag the build descends from: `2.2.1-7+gc1a17793` pins `2.2.1`.
///
/// `None` when the build carries no release base at all — a tagless checkout
/// stamps the workspace manifest's placeholder, which names no release. The
/// generated Dockerfile then resolves the newest release at build time instead
/// of pinning a tag that was never published.
#[must_use]
pub fn default_cli_version() -> Option<String> {
    release_base(tcl_version::VERSION)
}

/// Reduce a stamped version to the release tag it descends from.
fn release_base(version: &str) -> Option<String> {
    let base = version.split('+').next().unwrap_or(version);
    let base = base.strip_suffix("-dirty").unwrap_or(base);
    // `git describe` counts commits since the tag: `2.2.1-7` came from `v2.2.1`.
    let base = match base.rsplit_once('-') {
        Some((tag, distance))
            if !distance.is_empty() && distance.chars().all(|c| c.is_ascii_digit()) =>
        {
            tag
        }
        _ => base,
    };
    (!base.is_empty() && base != MANIFEST_PLACEHOLDER_VERSION).then(|| base.to_string())
}

/// Normalise a release version for the `TCL_LSP_VERSION` build argument: no
/// leading `v` (the fetch step adds it back when it builds the tag).
fn normalise_cli_version(version: &str) -> String {
    version.trim().trim_start_matches('v').to_string()
}

/// Return the Dockerfile snippet that downloads the native `tcl` CLI for the
/// build's target architecture and verifies it against the release's
/// `SHA256SUMS` before installing it at `/usr/local/bin/tcl`.
///
/// `cli_version` pins a release; `None` defers to [`default_cli_version`], and
/// when that is also `None` the emitted `TCL_LSP_VERSION` build argument is
/// empty and the snippet resolves the newest release itself. Either way the
/// version stays overridable at build time with
/// `docker build --build-arg TCL_LSP_VERSION=…`.
///
/// The trust model mirrors `scripts/tcl-mcp`: an asset missing from
/// `SHA256SUMS`, or one whose hash does not match, fails the build rather than
/// landing an unverified binary in the image.
#[must_use]
pub fn tcl_cli_install_recipe(cli_version: Option<&str>) -> String {
    let pinned = cli_version
        .map(normalise_cli_version)
        .or_else(default_cli_version)
        .unwrap_or_default();

    let arch_cases = RELEASE_TRIPLES
        .iter()
        .map(|(target_arch, uname, triple)| {
            // The two spellings coincide on some architectures (riscv64);
            // emitting both would be a duplicate `case` pattern.
            let pattern = if target_arch == uname {
                (*target_arch).to_string()
            } else {
                format!("{target_arch} | {uname}")
            };
            format!("        {pattern}) triple=\"{triple}\" ;; \\")
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "ARG TCL_LSP_VERSION={pinned}\n\
         ARG TARGETARCH\n\
         RUN set -eu; \\\n\
         \x20   arch=\"${{TARGETARCH:-$(uname -m)}}\"; \\\n\
         \x20   case \"$arch\" in \\\n\
         {arch_cases}\n\
         \x20       *) echo \"no tcl release asset for $arch\" >&2; exit 1 ;; \\\n\
         \x20   esac; \\\n\
         \x20   if [ -n \"${{TCL_LSP_VERSION:-}}\" ]; then \\\n\
         \x20       tag=\"v${{TCL_LSP_VERSION#v}}\"; \\\n\
         \x20   else \\\n\
         \x20       tag=\"$(curl -fsSL \"https://api.github.com/repos/{RELEASE_REPO}/releases?per_page=1\" \\\n\
         \x20           | grep -m1 '\"tag_name\"' \\\n\
         \x20           | sed -E 's/.*\"tag_name\": *\"([^\"]+)\".*/\\1/')\"; \\\n\
         \x20       [ -n \"$tag\" ] || {{ echo \"cannot resolve the latest {RELEASE_REPO} release\" >&2; exit 1; }}; \\\n\
         \x20   fi; \\\n\
         \x20   base=\"https://github.com/{RELEASE_REPO}/releases/download/$tag\"; \\\n\
         \x20   curl -fsSL \"$base/tcl-$triple\" -o /usr/local/bin/tcl; \\\n\
         \x20   curl -fsSL \"$base/SHA256SUMS\" -o /tmp/SHA256SUMS; \\\n\
         \x20   want=\"$(awk -v a=\"tcl-$triple\" '$2 == a || $2 == \"*\"a {{ print $1; exit }}' /tmp/SHA256SUMS)\"; \\\n\
         \x20   [ -n \"$want\" ] || {{ echo \"tcl-$triple is not listed in $tag SHA256SUMS\" >&2; exit 1; }}; \\\n\
         \x20   echo \"$want  /usr/local/bin/tcl\" | sha256sum -c -; \\\n\
         \x20   rm -f /tmp/SHA256SUMS; \\\n\
         \x20   chmod +x /usr/local/bin/tcl; \\\n\
         \x20   tcl --version"
    )
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

    let base_comment = match (spec.base_image.as_str(), family.as_str()) {
        (DEFAULT_BASE_IMAGE, _) => DEFAULT_BASE_IMAGE_COMMENT,
        (_, "alpine") => MUSL_BASE_IMAGE_COMMENT,
        _ => ALTERNATE_GLIBC_BASE_IMAGE_COMMENT,
    };
    lines.extend(base_comment.map(str::to_string));
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
        lines.push("# Prerequisites for fetching and verifying the tcl CLI".to_string());
        lines.push(cli_prereq_recipe(&spec.base_image)?);
        lines.push(String::new());

        lines.push("# Install the native tcl CLI from a verified release asset".to_string());
        lines.push(tcl_cli_install_recipe(spec.cli_version.as_deref()));
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
            "RUN tcl venv create .venv --tcl {}",
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
        lines.push("RUN if [ -f tclpkg.lock ]; then tcl pkg install --frozen; fi".to_string());
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
    fn default_version_strips_describe_and_build_metadata() {
        assert_eq!(release_base("2.2.1+gc1a17793").as_deref(), Some("2.2.1"));
        assert_eq!(release_base("2.2.1-7+gc1a17793").as_deref(), Some("2.2.1"));
        assert_eq!(
            release_base("2.2.1+gc1a17793.dirty").as_deref(),
            Some("2.2.1")
        );
        assert_eq!(release_base("2.2.1-dirty").as_deref(), Some("2.2.1"));
        // A real pre-release segment is part of the tag, not a describe count.
        assert_eq!(release_base("2.3.0-rc1").as_deref(), Some("2.3.0-rc1"));
        // The manifest placeholder names no release, so nothing is pinned.
        assert_eq!(release_base("0.1.0+gc1a17793"), None);
    }

    #[test]
    fn cli_install_recipe_pins_and_verifies() {
        let recipe = tcl_cli_install_recipe(Some("2.2.1"));
        assert!(recipe.starts_with("ARG TCL_LSP_VERSION=2.2.1\nARG TARGETARCH\n"));
        // A caller-supplied `v` prefix must not double up in the tag.
        assert!(tcl_cli_install_recipe(Some("v2.2.1")).starts_with("ARG TCL_LSP_VERSION=2.2.1\n"));
        assert!(
            recipe
                .contains("base=\"https://github.com/bitwisecook/tcl-lsp/releases/download/$tag\"")
        );
        assert!(recipe.contains("$base/tcl-$triple"));
        assert!(recipe.contains("$base/SHA256SUMS"));
        assert!(recipe.contains("sha256sum -c -"));
        assert!(recipe.contains("chmod +x /usr/local/bin/tcl"));
        assert!(recipe.ends_with("tcl --version"));
        for (target_arch, _, triple) in RELEASE_TRIPLES {
            assert!(recipe.contains(triple), "{triple} missing from arch case");
            assert!(recipe.contains(target_arch));
        }
        // Nothing Python-shaped survives.
        assert!(!recipe.contains("python"));
        assert!(!recipe.contains(".pyz"));
    }

    #[test]
    fn cli_install_recipe_without_a_pin_resolves_the_latest_release() {
        // `tcl_cli_install_recipe(None)` follows the build's own version, which
        // varies; drive the empty-pin shape through the shared formatter.
        let recipe = tcl_cli_install_recipe(Some(""));
        assert!(recipe.starts_with("ARG TCL_LSP_VERSION=\n"));
        assert!(recipe.contains("api.github.com/repos/bitwisecook/tcl-lsp/releases?per_page=1"));
    }

    #[test]
    fn cli_prereqs_carry_no_python() {
        for image in ["debian:bookworm-slim", "fedora:39"] {
            let recipe = cli_prereq_recipe(image).unwrap();
            assert!(recipe.contains("curl"), "{image}: no curl");
            assert!(!recipe.contains("python"), "{image}: still installs python");
        }
    }

    #[test]
    fn every_advertised_cli_family_has_a_recipe() {
        let families = cli_capable_families();
        assert!(!families.is_empty());
        for family in &families {
            assert!(
                cli_prereq_recipe(family).is_ok(),
                "{family} is advertised as CLI-capable but has no prerequisite recipe"
            );
        }
        assert!(!families.iter().any(|f| f == "alpine"));
        // Sorted, because this is a user-facing listing.
        let mut sorted = families.clone();
        sorted.sort();
        assert_eq!(families, sorted);
    }

    #[test]
    fn alpine_cannot_carry_the_glibc_cli() {
        let err = cli_prereq_recipe("alpine:3.19").unwrap_err().to_string();
        assert!(err.contains("musl"), "unhelpful alpine error: {err}");
        assert!(
            err.contains("build tcl-lsp from source"),
            "no source-build alternative named: {err}"
        );

        // Asking for the CLI on alpine fails ...
        let with_cli = DockerfileSpec {
            base_image: "alpine:3.19".to_string(),
            ..Default::default()
        };
        assert!(generate_dockerfile(&with_cli).is_err());

        // ... but a Tcl-only alpine image still generates.
        let tcl_only = DockerfileSpec {
            install_packages: false,
            ..with_cli
        };
        let out = generate_dockerfile(&tcl_only).unwrap();
        assert!(out.starts_with(
            "# Published tcl-lsp Linux binaries cannot run here because Alpine uses musl.\n\
             # Build tcl-lsp from source inside the image if its tools are required.\n\
             FROM alpine:3.19\n"
        ));
        assert!(out.contains("RUN apk add --no-cache tcl"));
        assert!(!out.contains("ARG TCL_LSP_VERSION"));
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
        assert!(out.starts_with(
            "# Published tcl-lsp Linux binaries require glibc, so Debian is the safe default.\n\
             # If Alpine/musl is required, build tcl-lsp from source inside the image.\n\
             FROM debian:bookworm-slim\n"
        ));
        assert!(out.contains("# Install Tcl 8.6"));
        assert!(out.contains("WORKDIR /app"));
        assert!(out.contains("COPY . ."));
        assert!(out.trim_end().ends_with("CMD [\"tclsh\"]"));
        // No CLI verbs are used, so no CLI is fetched.
        assert!(!out.contains("ARG TCL_LSP_VERSION"));
    }

    #[test]
    fn generate_installs_the_native_cli() {
        let spec = DockerfileSpec {
            cli_version: Some("2.2.1".to_string()),
            create_venv: true,
            entrypoint: Some("main.tcl".to_string()),
            ..Default::default()
        };
        let out = generate_dockerfile(&spec).unwrap();
        assert!(out.contains("ARG TCL_LSP_VERSION=2.2.1"));
        assert!(out.contains("triple=\"x86_64-unknown-linux-gnu\""));
        assert!(out.contains("sha256sum -c -"));
        assert!(out.contains("RUN tcl venv create .venv --tcl 8.6"));
        assert!(out.contains("RUN if [ -f tclpkg.lock ]; then tcl pkg install --frozen; fi"));
        assert!(out.trim_end().ends_with("CMD [\"tclsh\", \"main.tcl\"]"));
        assert!(
            !out.to_lowercase().contains("python"),
            "generated Dockerfile still mentions Python:\n{out}"
        );
        assert!(!out.contains(".pyz"));
    }

    #[test]
    fn generate_carries_no_python_on_any_family() {
        for image in ["debian:bookworm-slim", "fedora:39"] {
            for version in SUPPORTED_TCL_VERSIONS {
                let spec = DockerfileSpec {
                    base_image: image.to_string(),
                    tcl_version: version.to_string(),
                    cli_version: Some("2.2.1".to_string()),
                    create_venv: true,
                    ..Default::default()
                };
                let out = generate_dockerfile(&spec).unwrap();
                assert!(
                    !out.to_lowercase().contains("python"),
                    "{image} tcl {version} still mentions Python"
                );
                assert!(out.contains("ARG TCL_LSP_VERSION=2.2.1"));
            }
        }
    }
}
