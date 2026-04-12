---
name: dockerfile-generate
description: >
  Generate a Dockerfile for a Tcl project targeting a specific base image
  and Tcl version. Analyses the project to detect tclpkg.tcl manifests,
  entry points, and dependencies, then produces a production-ready
  Dockerfile with Python 3, the tcl CLI zipapp, and tcl pkg/venv integration.
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
2. Install **Python 3** (required to run the tcl CLI tool)
3. Download the **tcl CLI zipapp** (`tcl-VERSION.pyz`) from GitHub releases
4. Run `tcl pkg sync --frozen` to install packages from `tclpkg.lock`
5. Optionally run `tcl venv create` to set up a virtual environment

This skill wraps that capability with AI-driven project analysis to produce
better results than the CLI alone.

### How it works inside the container

The generated Dockerfile downloads the self-contained `tcl-VERSION.pyz` zipapp
from `https://github.com/bitwisecook/tcl-lsp/releases/`. This zipapp only
needs a Python 3.10+ interpreter — no pip installs required. It provides the
full `tcl pkg` and `tcl venv` toolchain:

```dockerfile
# Install Python 3 (required by the tcl CLI tool)
RUN apt-get update && apt-get install -y --no-install-recommends python3 curl ca-certificates && ...

# Download the tcl CLI zipapp from GitHub releases
RUN curl -fSL "https://github.com/bitwisecook/tcl-lsp/releases/download/v1.6.0/tcl-1.6.0.pyz" \
        -o /usr/local/bin/tcl.pyz && \
    chmod +x /usr/local/bin/tcl.pyz

# Install Tcl packages from lockfile
RUN if [ -f tclpkg.lock ]; then python3 /usr/local/bin/tcl.pyz pkg sync --frozen; fi

# Create Tcl virtual environment
RUN python3 /usr/local/bin/tcl.pyz venv create .venv --tcl 8.6
```

### Available Tcl versions
- **8.4** — legacy, built from source on all platforms
- **8.5** — legacy, built from source on all platforms
- **8.6** — current stable, available via OS package managers
- **9.0** — latest, built from source on most platforms

### Supported base-image families
- **debian** (also ubuntu, buildpack-deps, slim variants)
- **alpine**
- **redhat** (also fedora, centos, rockylinux, almalinux, amazonlinux)

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
   - Run the recipe lookup to verify:
     ```bash
     uv run --no-dev python -c "
     from tclpkg.docker import tcl_install_recipe, detect_image_family, python_install_recipe
     family = detect_image_family('IMAGE_NAME')
     print(f'Family: {family}')
     print(tcl_install_recipe('IMAGE_NAME', 'TCL_VERSION'))
     print(python_install_recipe('IMAGE_NAME'))
     "
     ```

3. **Generate the Dockerfile** using the CLI tool as a starting point:
   ```bash
   uv run --no-dev python -m explorer.tcl_cli docker create BASE_IMAGE \
       --tcl-version VERSION \
       --output Dockerfile \
       --force \
       [--entrypoint main.tcl] \
       [--venv] \
       [--cli-version 1.6.0] \
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
   __pycache__
   *.pyc
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
- The `tclpkg.docker` module in `tclpkg/docker.py` has the recipe database
- Use `--no-packages` to skip the tcl CLI download + pkg sync (pure Tcl image)
- Use `--cli-version` to pin a specific tcl-lsp release version

$ARGUMENTS
