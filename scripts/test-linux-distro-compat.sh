#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later
# Run the released x86_64 GNU/Linux server in representative supported distro
# userspaces. This checks the real release bytes, complementing the ELF symbol
# ceiling asserted by verify-glibc-baseline.sh.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
binary="${1:-$root/target/x86_64-unknown-linux-gnu/release/tcl-lsp-server}"
binary="$(realpath "$binary")"

if [[ ! -x "$binary" ]]; then
    printf 'error: executable server binary not found: %s\n' "$binary" >&2
    exit 2
fi
"$root/scripts/verify-glibc-baseline.sh" 2.28 "$binary"

engine="${CONTAINER_ENGINE:-}"
if [[ -z "$engine" ]]; then
    if command -v docker >/dev/null 2>&1; then
        engine=docker
    elif command -v podman >/dev/null 2>&1; then
        engine=podman
    else
        printf 'error: docker or podman is required for the distro compatibility test\n' >&2
        exit 2
    fi
fi
command -v "$engine" >/dev/null 2>&1 || {
    printf 'error: CONTAINER_ENGINE=%s is not available\n' "$engine" >&2
    exit 2
}

if [[ -n "${TCL_LSP_DISTRO_IMAGES:-}" ]]; then
    read -r -a images <<<"$TCL_LSP_DISTRO_IMAGES"
else
    # A compact ABI-family matrix, current as of 2026-09-01. EL8 covers the
    # RHEL/Rocky/Alma family; Oracle is retained explicitly because it is a
    # common downstream with its own release channel. Rolling distributions
    # prove forward compatibility with a much newer glibc.
    images=(
        docker.io/library/ubuntu:22.04
        docker.io/library/debian:12-slim
        registry.access.redhat.com/ubi8/ubi-minimal:8.10
        container-registry.oracle.com/os/oraclelinux:8-slim
        docker.io/amazonlinux:2023
        registry.opensuse.org/opensuse/leap:16.0
        docker.io/library/fedora:44
        docker.io/archlinux:base
    )
fi

frame() {
    local body="$1"
    printf 'Content-Length: %d\r\n\r\n%s' "${#body}" "$body"
}

handshake_input() {
    frame '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}'
}

failed=0
for image in "${images[@]}"; do
    printf '%-6s %s\n' PULL "$image"
    "$engine" pull --quiet "$image" >/dev/null
    output="$({
        handshake_input | timeout 60 "$engine" run --rm -i --network=none \
            -v "$binary:/tcl-lsp-server:ro" \
            "$image" /tcl-lsp-server
    } 2>&1 || true)"
    if grep -q '"capabilities"' <<<"$output"; then
        printf '%-6s %s\n' PASS "$image"
    else
        printf '%-6s %s (no initialize response)\n' FAIL "$image" >&2
        printf '%s\n' "$output" >&2
        failed=1
    fi
done

exit "$failed"
