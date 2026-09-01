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

# The managed wrapper directory shared by setup and focused conformance
# runners. Prefer a conventional directory already named on PATH; callers may
# select another without changing this adapter's release facts.
tcl_reference_default_bin_dir() {
    printf '%s\n' "${PATH:-}" | tr ':' '\n' | awk -v user_bin="${HOME:-}/.local/bin" '
        $0 == user_bin || $0 == "/usr/local/bin" {
            print
            found = 1
            exit
        }
        END {
            if (!found) {
                print "/usr/local/bin"
            }
        }
    '
}

tcl_reference_bin_dir() {
    if [ -n "${TCL_LSP_TCL_BIN_DIR:-}" ]; then
        printf '%s\n' "$TCL_LSP_TCL_BIN_DIR"
    else
        tcl_reference_default_bin_dir
    fi
}

# Print one interpreter's reported patchlevel. TCL_LIBRARY is deliberately
# absent so a stale cross-release library export cannot change the probe.
tcl_reference_tclsh_patchlevel() {
    [ -x "$1" ] || return 1
    printf 'puts [info patchlevel]\n' | env -u TCL_LIBRARY "$1" 2>/dev/null
}

tcl_reference_tclsh_reports_patchlevel() {
    tcl_reference_actual=$(tcl_reference_tclsh_patchlevel "$1") || return 1
    [ "$tcl_reference_actual" = "$2" ]
}

# Resolve an exact reference interpreter. An explicit Rust-oracle override is
# a promise and therefore fails closed when stale. Otherwise prefer an exact
# PATH command, then the exact wrapper at the known managed path. The latter
# is what lets conformance run without overwriting a stale system executable
# that appears earlier on the user's PATH.
tcl_reference_resolve_tclsh() {
    tcl_reference_release=$1
    tcl_reference_expected=$(tcl_reference_patchlevel "$tcl_reference_release") || return 2
    tcl_reference_compact=$(printf '%s' "$tcl_reference_release" | tr -d '.')
    tcl_reference_override_name="TCL_LSP_TCLSH${tcl_reference_compact}"
    eval "tcl_reference_override=\${$tcl_reference_override_name-}"
    if [ -n "$tcl_reference_override" ]; then
        if tcl_reference_tclsh_reports_patchlevel "$tcl_reference_override" "$tcl_reference_expected"; then
            printf '%s\n' "$tcl_reference_override"
            return 0
        fi
        tcl_reference_actual=$(tcl_reference_tclsh_patchlevel "$tcl_reference_override" 2>/dev/null || true)
        echo "tcl-reference-toolchains: $tcl_reference_override_name=$tcl_reference_override reports ${tcl_reference_actual:-no usable Tcl}, expected Tcl $tcl_reference_expected" >&2
        return 2
    fi

    tcl_reference_command=$(command -v "tclsh${tcl_reference_release}" 2>/dev/null || true)
    if tcl_reference_tclsh_reports_patchlevel "$tcl_reference_command" "$tcl_reference_expected"; then
        printf '%s\n' "$tcl_reference_command"
        return 0
    fi

    tcl_reference_managed="$(tcl_reference_bin_dir)/tclsh${tcl_reference_release}"
    if tcl_reference_tclsh_reports_patchlevel "$tcl_reference_managed" "$tcl_reference_expected"; then
        printf '%s\n' "$tcl_reference_managed"
        return 0
    fi
    return 1
}
