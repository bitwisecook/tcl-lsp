#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

set -euo pipefail

demo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${demo_dir}/../.." && pwd)"
cd "${repo_root}"

if [ "$(uname -s)" != Linux ]; then
    echo "run-kernel-xdp-demo.sh requires Linux" >&2
    exit 1
fi
for tool in cargo bpftool readelf; do
    if ! command -v "${tool}" >/dev/null 2>&1; then
        echo "missing required tool: ${tool}" >&2
        exit 1
    fi
done
if ! mountpoint -q /sys/fs/bpf; then
    echo "/sys/fs/bpf is not mounted; run: sudo mount -t bpf bpf /sys/fs/bpf" >&2
    exit 1
fi

source scripts/dev/agent-build-env.sh
cargo build -p bpf-tcl
bpf_bin="${CARGO_TARGET_DIR}/debug/bpf-tcl"

demo_tmp="$(mktemp -d "${TMPDIR:-/tmp}/bpf-tcl-kernel-demo.XXXXXX")"
elf_path="${demo_tmp}/xdp-pass.o"
packet_path="${demo_tmp}/packet.bin"
packet_out_path="${demo_tmp}/packet-out.bin"
run_log_path="${demo_tmp}/run.log"
pin_path="/sys/fs/bpf/bpf-tcl-xdp-pass-$$"

cleanup() {
    if sudo test -e "${pin_path}"; then
        sudo unlink "${pin_path}"
    fi
    if [ -e "${elf_path}" ]; then
        unlink "${elf_path}"
    fi
    if [ -e "${packet_path}" ]; then
        unlink "${packet_path}"
    fi
    if [ -e "${packet_out_path}" ]; then
        unlink "${packet_out_path}"
    fi
    if [ -e "${run_log_path}" ]; then
        unlink "${run_log_path}"
    fi
    rmdir "${demo_tmp}" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

if sudo test -e "${pin_path}"; then
    echo "refusing to replace existing pin: ${pin_path}" >&2
    exit 1
fi

"${bpf_bin}" compile "${demo_dir}/xdp-kernel-pass.bpftcl" \
    --target kernel-xdp --emit elf --program 0 -o "${elf_path}"
readelf -h -S -s "${elf_path}"

dd if=/dev/zero of="${packet_path}" bs=64 count=1 status=none
sudo bpftool -d prog load "${elf_path}" "${pin_path}" type xdp
sudo bpftool prog run pinned "${pin_path}" data_in "${packet_path}" \
    data_out "${packet_out_path}" repeat 1 | tee "${run_log_path}"
if ! grep -Fq "Return value: 2" "${run_log_path}"; then
    echo "expected BPF_PROG_TEST_RUN to return 2 (XDP_PASS)" >&2
    exit 1
fi

echo "Verified return value: 2 (XDP_PASS)"
echo "The program was verifier-loaded and test-run, but not attached to an interface."
