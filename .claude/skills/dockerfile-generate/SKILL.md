---
name: dockerfile-generate
description: >
  Generate a Dockerfile for a Tcl project targeting a specific base image
  and Tcl version. Analyses the project to detect tclpkg.tcl manifests,
  entry points, and dependencies, then produces a production-ready
  Dockerfile. Use when creating Docker containers for Tcl applications,
  setting up CI images, or containerising Tcl projects.
allowed-tools: Bash, Read, Write, Edit, Glob, Grep
---

# Dockerfile Generate

Generate a production-ready Dockerfile for a Tcl project, tailored to the
user's chosen base image and Tcl version.

## Context

The `tcl docker create` CLI verb provides deterministic Dockerfile generation
with built-in recipes for Debian/Ubuntu, Alpine, and RHEL/Fedora families.
This skill wraps that capability with AI-driven project analysis to produce
better results than the CLI alone.

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
     from tclpkg.docker import tcl_install_recipe, detect_image_family
     family = detect_image_family('IMAGE_NAME')
     recipe = tcl_install_recipe('IMAGE_NAME', 'TCL_VERSION')
     print(f'Family: {family}')
     print(recipe)
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
       [--extra-package PACKAGE]
   ```

4. **Customise the generated Dockerfile** based on project analysis:
   - If Tk is used, add display-related packages (xvfb, tk8.6, etc.)
   - If the project has C extensions, add build-essential/gcc
   - If there are test files, consider a multi-stage build
   - Add `.dockerignore` if one doesn't exist
   - Optimise layer caching (copy dependency files before source)

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
- For unknown/custom base images, fall back to Debian-style recipes and warn the user
- Always prefer OS package manager installs over building from source when possible
- The `tclpkg.docker` module in `tclpkg/docker.py` has the recipe database

$ARGUMENTS
