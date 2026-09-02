---
name: dockerfile-generate
description: >
  Generate a Dockerfile for a Tcl project targeting a specific base image
  and Tcl version. Analyses the project to detect tclpkg.tcl manifests,
  entry points, and dependencies, then produces a production-ready
  Dockerfile with the native tcl CLI binary and tcl pkg/venv integration.
  Use when creating Docker containers for Tcl applications, setting up CI
  images, or containerising Tcl projects.
allowed-tools: Bash, Read, Write, Edit, Glob, Grep
---

# Dockerfile Generate

Generate a production-ready Dockerfile for a Tcl project, tailored to the
user's chosen base image and Tcl version.

## Context

The `tcl docker create` CLI verb generates Dockerfiles that:

1. Install the requested **Tcl version** (8.4, 8.5, 8.6, 9.0)
2. Download the **native `tcl` CLI binary** for the build's target
   architecture from a GitHub release, verified against the release's
   `SHA256SUMS`
3. Run `tcl pkg install --frozen` to install packages from `tclpkg.lock`
4. Optionally run `tcl venv create` to set up a virtual environment

Nothing in the image needs a Python interpreter — the whole toolchain is the
single static-ish `tcl` binary. This skill wraps that capability with
AI-driven project analysis to produce better results than the CLI alone.

### How it works inside the container

The generated Dockerfile fetches `tcl-<triple>` from
`https://github.com/bitwisecook/tcl-lsp/releases/` and refuses to install it
unless its SHA-256 matches the release's `SHA256SUMS` entry — the same trust
model `scripts/tcl-mcp` uses for the MCP binary. The architecture comes from
BuildKit's `TARGETARCH` (falling back to `uname -m` under the classic
builder), so one Dockerfile works for `linux/amd64`, `linux/arm64`, and
`linux/riscv64`:

```dockerfile
# Fetch and verify the native tcl CLI release asset
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

# Install the native tcl CLI from a verified release asset
ARG TCL_LSP_VERSION=2.2.1
ARG TARGETARCH
RUN set -eu; \
    arch="${TARGETARCH:-$(uname -m)}"; \
    case "$arch" in \
        amd64 | x86_64) triple="x86_64-unknown-linux-gnu" ;; \
        arm64 | aarch64) triple="aarch64-unknown-linux-gnu" ;; \
        riscv64) triple="riscv64gc-unknown-linux-gnu" ;; \
        *) echo "no tcl release asset for $arch" >&2; exit 1 ;; \
    esac; \
    ... curl the asset + SHA256SUMS, sha256sum -c, chmod +x, tcl --version

# Install Tcl packages from lockfile
RUN if [ -f tclpkg.lock ]; then tcl pkg install --frozen; fi

# Create Tcl virtual environment
RUN tcl venv create .venv --tcl 8.6
```

`ARG TCL_LSP_VERSION` defaults to the release the generating `tcl` binary was
built from, and stays overridable at build time:

```bash
docker build --build-arg TCL_LSP_VERSION=2.2.1 -t myapp .   # pin
docker build --build-arg TCL_LSP_VERSION= -t myapp .        # newest release
```

The final `tcl --version` in that layer is deliberate: a binary that cannot
start fails the build instead of shipping.

### Available Tcl versions
- **8.4** — legacy, built from source on all platforms
- **8.5** — legacy, built from source on all platforms
- **8.6** — current stable, available via OS package managers
- **9.0** — latest, built from source on most platforms

### Supported base-image families
- **debian** (also ubuntu, buildpack-deps, slim variants) — Tcl **and** CLI
- **redhat** (also fedora, centos, rockylinux, almalinux, amazonlinux) —
  Tcl **and** CLI
- **alpine** — Tcl only

Alpine is musl, and every published Linux `tcl` asset is glibc-linked (the
release matrix has no musl leg). `gcompat` does not close the gap — it
re-exports neither `fcntl64` nor `__res_init`, so the binary dies in the
dynamic loader. `tcl docker create alpine:… ` therefore **errors** as soon as
a CLI verb is wanted. For an Alpine image, either drop the CLI
(`--no-packages`, no `--venv`) for a plain Tcl runtime, or pick a glibc base.

## Steps

1. **Analyse the project** to understand what we're containerising:
   - Check for `tclpkg.tcl` manifest (package dependencies)
   - Check for `tclpkg.lock` lockfile (frozen dependencies)
   - Look for entry point scripts (main.tcl, app.tcl, or `entry` in manifest)
   - Scan for Tk usage (may need X11/display packages)
   - Check for any existing Dockerfile to understand prior intent

2. **Determine the Tcl install strategy** based on the base image:
   - For Tcl 8.6 on Debian/Ubuntu/Alpine/RHEL: use OS package manager
   - For Tcl 8.4, 8.5, 9.0 or exotic images: build from source
   - Inspect the recipes with the CLI itself:
     ```bash
     tcl docker info                                  # families, versions, CLI targets
     tcl docker recipe IMAGE --tcl-version VERSION    # the Tcl install layer
     tcl docker recipe IMAGE --cli                    # the tcl CLI install layer
     ```

3. **Generate the Dockerfile**:
   ```bash
   tcl docker create BASE_IMAGE \
       --tcl-version VERSION \
       --output Dockerfile \
       --force \
       [--entrypoint main.tcl] \
       [--venv] \
       [--cli-version 2.2.1] \
       [--extra-package PACKAGE]
   ```

4. **Customise the generated Dockerfile** based on project analysis:
   - If Tk is used, add display-related packages (xvfb, tk8.6, etc.)
   - If the project has C extensions, add build-essential/gcc
   - If there are test files, consider a multi-stage build
   - Add `.dockerignore` if one doesn't exist
   - Optimise layer caching (copy tclpkg.tcl + tclpkg.lock before full COPY)

5. **Review and refine** the Dockerfile:
   - Ensure the image is as small as possible (clean up build deps)
   - Verify the entrypoint is correct
   - Add health checks if appropriate
   - Consider security (non-root user, read-only filesystem)

6. **Create a `.dockerignore`** if one doesn't exist:
   ```
   .git
   .venv
   .vscode
   target/
   tmp/
   .claude/
   ```

7. **Report** the generated files and provide build/run instructions.

## Output Format

After generation, provide:
- Path to the generated Dockerfile
- Base image and Tcl version used
- Build command: `docker build -t <project-name> .`
- Run command: `docker run --rm <project-name>`
- Any caveats or manual steps needed

## Notes

- If the user doesn't specify a base image, default to `debian:bookworm-slim`
- If the user doesn't specify a Tcl version, default to `8.6`
- For unknown/custom base images, fall back to Debian-style recipes and warn
- Always prefer OS package manager installs over building from source when possible
- The recipe database is `rust/tcl-pkg/src/docker.rs`
- Use `--no-packages` to skip the tcl CLI download entirely (pure Tcl image)
- Use `--cli-version` to pin a specific tcl-lsp release; pass an empty value
  to resolve the newest release at build time
- Building the image needs network access to `github.com` (release assets)
  and, when unpinned, `api.github.com` (tag resolution)

$ARGUMENTS
