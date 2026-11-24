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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#

set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
source scripts/dev/tcl-reference-toolchains.sh
tcl_reference_load_toolchains "$root"

fetch=.claude/skills/fetch-tcl-source/fetch_tcl_source.sh
bash "$fetch" all
bash "$fetch" tkall
make ensure-tcl-deps ensure-tcllib-source
export TCL_LSP_JIM_ROOT=${TCL_LSP_JIM_ROOT:-"$root/tmp/jimtcl-oracle"}
bash scripts/dev/ensure-jim-oracle.sh
export TCL_LSP_JIMSH="$TCL_LSP_JIM_ROOT/jimsh"
export TCL_LSP_TCLLIB_MODULES="$root/tmp/tcllib-2.0/modules"

flags='-O2 -fcommon -Wno-implicit-int -Wno-implicit-function-declaration -Wno-incompatible-pointer-types -Wno-int-conversion'
for release in $(tcl_reference_releases); do
    patch=$(tcl_reference_patchlevel "$release")
    compact=${release//./}
    tcl="$root/tmp/tcl$patch/unix"
    tk="$root/tmp/tk$patch/unix"
    if [[ ! -x "$tcl/tclsh" || ! -f "$tcl/tclConfig.sh" ]]; then
        (cd "$tcl" && CFLAGS="$flags -std=gnu17" ./configure --disable-shared && make -j2)
    fi
    library="libtk$release.so"
    if [[ "$release" == 9.* ]]; then library="libtcl9tk$release.so"; fi
    if [[ ! -f "$tk/$library" ]]; then
        (cd "$tk" && CFLAGS="$flags -std=gnu17" ./configure --with-tcl="$tcl" --disable-xft && make -j2 "$library")
    fi
    export "TCL_LSP_TCLSH$compact=$tcl/tclsh"
    export "TCL_LSP_TK$compact=$tk/$library"
done

ulimit -c 0
cargo xtask definitions-gate

# The Tk and Jim environments have no execution backend in the VM or runtime.
# For their portable Tcl counterparts compare the exact messages on both.
target=${CARGO_TARGET_DIR:-"$root/target"}
runtime_target="$target/definition-runtime"
cargo build --locked --target-dir "$target" -p tcl-vm-cli --bin tclvm
tommath="$root/tmp/tcl$(tcl_reference_patchlevel 9.0)/libtommath"
TCL_TOMMATH_DIR="$tommath" cargo build --locked --manifest-path runtime/rust/Cargo.toml \
    --target-dir "$runtime_target" --example run_script
usage="$root/docs/work/name-handling/handover/oo-dispatch-probe/definitions/engine-usage.tcl"
output="${TCL_LSP_DEFINITIONS_WORK:-"$root/tmp/command-definitions"}"
for release in $(tcl_reference_releases); do
    compact=${release//./}
    shell_variable="TCL_LSP_TCLSH$compact"
    "${!shell_variable}" "$usage" > "$output/usage-$release-c-tcl.tsv"
    "$target/debug/tclvm" --tcl-version "$release" "$usage" > "$output/usage-$release-vm.tsv"
    "$runtime_target/debug/examples/run_script" --quiet --tcl-version "$release" "$usage" \
        > "$output/usage-$release-runtime.tsv"
    diff -u "$output/usage-$release-c-tcl.tsv" "$output/usage-$release-vm.tsv"
    diff -u "$output/usage-$release-c-tcl.tsv" "$output/usage-$release-runtime.tsv"
done

# Retain all distinct observed C Tcl usage lines beside their engine answers.
# Run both engines even when a command is missing or an observation times out.
probe="$root/docs/work/name-handling/handover/oo-dispatch-probe/definitions/capture-engine-usage.tcl"
workers=()
for release in $(tcl_reference_releases); do
    compact=${release//./}
    shell_variable="TCL_LSP_TCLSH$compact"
    patch=$(tcl_reference_patchlevel "$release")
    (
        cd "$output/tcl-$patch"
        "${!shell_variable}" "$probe" usage.tsv usage-vm.tsv "$target/debug/tclvm" "$release" 0
        "${!shell_variable}" "$probe" usage.tsv usage-runtime.tsv "$runtime_target/debug/examples/run_script" "$release" 1
    ) &
    workers+=("$!")
done
for worker in "${workers[@]}"; do wait "$worker"; done

"${TCL_LSP_TCLSH86}" "${probe%/*}/check-usage.tcl" "$output" \
    "${probe%/*}/known-usage-differences.tsv" "$output/usage-differences.tsv"
