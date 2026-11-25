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

# Run the probe fixtures as conformance rows (`cargo xtask conformance`)
# against the pinned C Tcl releases and jimsh that
# run-definition-oracles.sh builds.

set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
source scripts/dev/tcl-reference-toolchains.sh
tcl_reference_load_toolchains "$root"

export TCL_LSP_JIM_ROOT=${TCL_LSP_JIM_ROOT:-"$root/tmp/jimtcl-oracle"}
bash scripts/dev/ensure-jim-oracle.sh
export TCL_LSP_JIMSH="$TCL_LSP_JIM_ROOT/jimsh"
for release in $(tcl_reference_releases); do
    patch=$(tcl_reference_patchlevel "$release")
    export "TCL_LSP_TCLSH${release//./}=$root/tmp/tcl$patch/unix/tclsh"
done

cargo xtask conformance "$@"
