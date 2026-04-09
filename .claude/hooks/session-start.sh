#!/bin/bash
# SessionStart hook for tcl-lsp.
#
# Claude Code on the web runs in a sandbox that ships without some
# system tools the Makefile depends on.  This hook installs anything
# missing so `make prep-pr`, `make test-ext`, and `make smoke-vsix`
# keep working across fresh sessions.
#
# Runs only in remote sessions — skips locally so developer machines
# are never touched.

set -euo pipefail

# Skip outside Claude Code on the web.
if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
    exit 0
fi

# List of required system packages.  Each is installed only when its
# canonical binary is missing, so the hook is idempotent and fast on
# warm containers.
declare -A REQUIRED_PKGS=(
    [rsync]="rsync"
)

missing_pkgs=()
for bin in "${!REQUIRED_PKGS[@]}"; do
    if ! command -v "$bin" >/dev/null 2>&1; then
        missing_pkgs+=("${REQUIRED_PKGS[$bin]}")
    fi
done

if [ "${#missing_pkgs[@]}" -eq 0 ]; then
    echo "session-start: all required system packages already present"
    exit 0
fi

echo "session-start: installing missing system packages: ${missing_pkgs[*]}"

export DEBIAN_FRONTEND=noninteractive
# apt-get needs root; in the Claude Code on the web sandbox we run as
# root so `sudo` is not strictly required, but fall back to it anyway
# so the hook works in more environments.
if [ "$(id -u)" -eq 0 ]; then
    APT="apt-get"
else
    APT="sudo apt-get"
fi

# Use -qq to suppress progress bars while still showing errors.
$APT update -qq
$APT install -y -qq --no-install-recommends "${missing_pkgs[@]}"

echo "session-start: done"
