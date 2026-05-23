#!/usr/bin/env bash
# Resolve the Tcl script library used by VM gates.
#
# The VM itself consumes TCL_LIBRARY; platform and checkout discovery live
# here so tests get the same path on laptops and Claude remote sessions.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OS="$(uname -s)"

declare -a CANDIDATES=()

add_candidate() {
    local path="${1:-}"
    if [ -n "$path" ]; then
        CANDIDATES+=("$path")
    fi
}

add_tclsh_library() {
    local tclsh="$1"
    local path=""

    if ! command -v "$tclsh" >/dev/null 2>&1; then
        return 0
    fi

    path="$(env -u TCL_LIBRARY "$tclsh" <<'TCL' 2>/dev/null || true
puts [info library]
TCL
)"
    add_candidate "$path"
}

valid_library() {
    local path="$1"

    [ -f "$path/init.tcl" ] || return 1
    if [ "${REQUIRE_TCLTEST:-0}" = "1" ]; then
        [ -f "$path/tcltest/tcltest.tcl" ] || return 1
    fi
}

# Existing environment wins.  This is the runtime contract consumed by
# tooling.vm.interp.TclInterp.
add_candidate "${TCL_LIBRARY:-}"

# Same source-tree location exported by .claude/hooks/session-start.sh after
# the SessionStart hook fetches the Tcl sources.
add_candidate "$REPO_ROOT/tmp/tcl9.0.3/library"

case "$OS" in
    Darwin)
        if command -v brew >/dev/null 2>&1; then
            for formula in tcl-tk tcl-tk@8; do
                prefix="$(brew --prefix "$formula" 2>/dev/null || true)"
                if [ -n "$prefix" ]; then
                    add_candidate "$prefix/lib/tcl9.0"
                    add_candidate "$prefix/lib/tcl8.6"
                fi
            done
        fi
        add_candidate "/opt/homebrew/opt/tcl-tk/lib/tcl9.0"
        add_candidate "/opt/homebrew/opt/tcl-tk/lib/tcl8.6"
        add_candidate "/usr/local/opt/tcl-tk/lib/tcl9.0"
        add_candidate "/usr/local/opt/tcl-tk/lib/tcl8.6"
        ;;
    Linux)
        add_candidate "/usr/share/tcltk/tcl9.0"
        add_candidate "/usr/share/tcltk/tcl8.6"
        add_candidate "/usr/lib/tcl9.0"
        add_candidate "/usr/lib/tcl8.6"
        add_candidate "/usr/lib64/tcl9.0"
        add_candidate "/usr/lib64/tcl8.6"
        ;;
esac

add_tclsh_library tclsh9.0
add_tclsh_library tclsh8.6
add_tclsh_library tclsh

for candidate in "${CANDIDATES[@]}"; do
    if valid_library "$candidate"; then
        printf '%s\n' "$candidate"
        exit 0
    fi
done

if [ "${REQUIRE_TCLTEST:-0}" = "1" ]; then
    echo "resolve-tcl-library: could not find a Tcl library containing init.tcl and tcltest/tcltest.tcl" >&2
else
    echo "resolve-tcl-library: could not find a Tcl library containing init.tcl" >&2
fi
echo "resolve-tcl-library: set TCL_LIBRARY or run make ensure-tcl-deps" >&2
exit 1
