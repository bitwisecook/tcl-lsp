#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

set -euo pipefail

demo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${demo_dir}/../.." && pwd)"
cd "${repo_root}"

source scripts/dev/agent-build-env.sh
cargo build -p bpf-tcl
bpf_bin="${CARGO_TARGET_DIR}/debug/bpf-tcl"

run_demo() {
    printf '\n$'
    printf ' %q' "$@"
    printf '\n'
    "$@"
}

printf 'BPF-Tcl userspace eBPF demos\n'
printf 'No root access or kernel attachment is used.\n'

run_demo "${bpf_bin}" check "${demo_dir}/socket-length.bpftcl"
run_demo "${bpf_bin}" run "${demo_dir}/socket-length.bpftcl" --packet 0102
run_demo "${bpf_bin}" run "${demo_dir}/socket-length.bpftcl" --packet 01020304

run_demo "${bpf_bin}" check "${demo_dir}/xdp-marker.bpftcl"
run_demo "${bpf_bin}" run "${demo_dir}/xdp-marker.bpftcl" --packet 7f000102
run_demo "${bpf_bin}" run "${demo_dir}/xdp-marker.bpftcl" --packet 01020304

run_demo "${bpf_bin}" run "${demo_dir}/map-counter.bpftcl" \
    --packet 0000000000000000 --repeat 3

run_demo "${bpf_bin}" check "${demo_dir}/priority-bundle.bpftcl"
run_demo "${bpf_bin}" compile "${demo_dir}/priority-bundle.bpftcl" \
    --emit asm --program 0

demo_out="$(mktemp -d "${TMPDIR:-/tmp}/bpf-tcl-demo.XXXXXX")"
elf_path="${demo_out}/xdp.o"
run_demo "${bpf_bin}" compile "${demo_dir}/priority-bundle.bpftcl" \
    --emit elf --program 0 -o "${elf_path}"

if command -v readelf >/dev/null 2>&1; then
    run_demo readelf -h -S -s "${elf_path}"
else
    printf '\nreadelf is not installed; ELF inspection skipped.\n'
fi

if command -v llvm-objdump >/dev/null 2>&1; then
    run_demo llvm-objdump -d "${elf_path}"
else
    printf '\nllvm-objdump is not installed; disassembly inspection skipped.\n'
fi

printf '\nELF inspection artefact: %s\n' "${elf_path}"
printf 'This object is structural output for inspection, not kernel-loadable.\n'
