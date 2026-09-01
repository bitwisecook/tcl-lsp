#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Shell adapter for tcl-dialect's language-neutral reference-toolchain
# manifest. It intentionally uses only POSIX shell syntax: ensure-test-deps
# runs under macOS's system Bash 3.2 as well as current Bash releases.

tcl_reference_load_toolchains() {
    TCL_REFERENCE_MANIFEST="${TCL_LSP_TCL_REFERENCE_MANIFEST:-$1/rust/tcl-dialect/data/reference-toolchains.tsv}"

    if [ ! -r "$TCL_REFERENCE_MANIFEST" ]; then
        echo "tcl-reference-toolchains: cannot read manifest $TCL_REFERENCE_MANIFEST" >&2
        return 1
    fi

    awk -F '\t' '
        BEGIN {
            expected[1] = "8.4"
            expected[2] = "8.5"
            expected[3] = "8.6"
            expected[4] = "9.0"
            expected[5] = "9.1"
        }
        /^#/ || NF == 0 { next }
        NF != 3 {
            printf "tcl-reference-toolchains: malformed row %d in %s\n", NR, FILENAME > "/dev/stderr"
            failed = 1
            next
        }
        {
            count++
            if ($1 != expected[count]) {
                printf "tcl-reference-toolchains: releases in %s must be 8.4, 8.5, 8.6, 9.0, 9.1\n", FILENAME > "/dev/stderr"
                failed = 1
            }
            if ($2 == "" || $3 == "") {
                printf "tcl-reference-toolchains: empty field on row %d in %s\n", NR, FILENAME > "/dev/stderr"
                failed = 1
            }
        }
        END {
            if (count != 5) {
                printf "tcl-reference-toolchains: expected five release rows in %s\n", FILENAME > "/dev/stderr"
                failed = 1
            }
            exit failed
        }
    ' "$TCL_REFERENCE_MANIFEST"
}

tcl_reference_releases() {
    awk -F '\t' '!/^#/ && NF != 0 { print $1 }' "$TCL_REFERENCE_MANIFEST"
}

tcl_reference_field() {
    awk -F '\t' -v release="$1" -v field="$2" '
        !/^#/ && NF != 0 && $1 == release {
            print $field
            found = 1
            exit
        }
        END {
            if (!found) {
                printf "tcl-reference-toolchains: unknown Tcl release %s\n", release > "/dev/stderr"
                exit 1
            }
        }
    ' "$TCL_REFERENCE_MANIFEST"
}

tcl_reference_patchlevel() {
    tcl_reference_field "$1" 2
}

tcl_reference_source_tag() {
    tcl_reference_field "$1" 3
}
