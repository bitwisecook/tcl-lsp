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
# SPDX-License-Identifier: AGPL-3.0-or-later

# ============================================================================
# SPIKE -- throwaway proof-of-concept. NOT the final design.
# ============================================================================
#
# API-compatibility validation: compile every extension in ../ext UNMODIFIED to
# a wasm object against our authored ../include headers, and report PASS/FAIL.
#
# This is the evidence that the authored tcl.h / tclOO.h / tclTomMath.h are
# API-compatible with real-world extension source. It compiles only (`-c`);
# linking/running is the job of the static-link and dynamic-link spikes.
#
# Compiler: `zig cc` (bundles wasi-libc, so extensions that use <stdio.h>,
# snprintf, <string.h>, etc. compile with no separate wasi sysroot). This also
# demonstrates the Rust-runtime + zig-cc hybrid toolchain.
#
# Exit 0 iff every extension compiles.
set -uo pipefail
cd "$(dirname "$0")"
mkdir -p build
INC=../include

# Real Tcl 9.0.3 dltest samples (byte-identical) + synthetic surface probes.
# embtest.c is intentionally excluded: it is an *embedder* (has main(), calls
# Tcl_FindExecutable / Tcl_InitSubsystems), not a loadable extension.
SOURCES=(
    ../ext/pkga.c          # core: command/obj/result/UTF
    ../ext/pkgb.c          # int/wide accessors, error line, AppendResult, EvalEx, snprintf
    ../ext/pkgc.c          # int accessor + string/int obj results
    ../ext/pkgd.c          # int accessor + string/int obj results
    ../ext/pkge.c          # error-returning init via Tcl_EvalEx
    ../ext/pkgt.c          # Tcl 9 Tcl_CreateObjCommand2 / Tcl_CreateObjTrace2 (Tcl_Size)
    ../ext/pkgua.c         # hash tables, thread-local data, load/unload, Tcl_SetVar2
    ../ext/pkgπ.c          # non-ASCII init-function naming (Pkgπ_Init)
    ../ext/pkgooa.c        # TclOO header + stub-table introspection (compile-only)
    ../ext/synth_surface.c # widened surface: Tcl_ObjType, channels, FS, threads, NRE
    ../ext/synth_tommath.c # tclTomMath.h bignum + Tcl_NewBignumObj
)

fail=0
printf '%-22s %s\n' "EXTENSION" "RESULT"
printf '%-22s %s\n' "---------" "------"
for src in "${SOURCES[@]}"; do
    name=$(basename "$src")
    obj="build/${name%.c}.o"
    if err=$(zig cc --target=wasm32-wasi -ffreestanding -fno-builtin \
                    -Wall -I "$INC" -c "$src" -o "$obj" 2>&1); then
        printf '%-22s %s\n' "$name" "PASS"
    else
        printf '%-22s %s\n' "$name" "FAIL"
        echo "$err" | sed 's/^/    /'
        fail=1
    fi
done

echo
if [ "$fail" -eq 0 ]; then
    echo "COMPILE-CHECK PASS — all extensions compile unmodified against include/"
else
    echo "COMPILE-CHECK FAIL"
fi
exit "$fail"
