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

# Turnkey runner for the issue #829 robustness stress suite: the
# direct-infrastructure salsa concurrency example, then all three LSP-API
# scenarios against the real server binary. See README.md in this directory
# for what each half exercises.
#
#   scripts/stress/run_all.sh
#   DURATION=120 DOCS=16 PROCS=600 scripts/stress/run_all.sh   # longer soak
#
# Exits non-zero if either suite fails. Requires only `cargo` and `python3`
# (standard library) — no extra dependencies.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

PROFILE="${PROFILE:-release}"
DURATION="${DURATION:-20}"
DOCS="${DOCS:-6}"
PROCS="${PROCS:-300}"
STARTUP_ITERATIONS="${STARTUP_ITERATIONS:-5}"

if [[ "$PROFILE" == "release" ]]; then
  CARGO_PROFILE_FLAG="--release"
  SERVER_BIN="target/release/tcl-lsp-server"
else
  CARGO_PROFILE_FLAG=""
  SERVER_BIN="target/debug/tcl-lsp-server"
fi

echo "==> Building tcl-lsp-server ($PROFILE)"
make rust-server "PROFILE=$PROFILE"

echo
echo "==> [1/2] Direct-infrastructure suite (no LSP): stress_concurrent_analysis"
STRESS_PROCS="$PROCS" \
STRESS_READERS=8 \
STRESS_WRITES="$((DURATION * 5))" \
STRESS_TIMEOUT_SECS="$((DURATION * 6 + 60))" \
  cargo run $CARGO_PROFILE_FLAG -p tcl-lsp-db --example stress_concurrent_analysis

echo
echo "==> [2/2] LSP-API suite: stress_lsp.py"
python3 "$ROOT_DIR/scripts/stress/stress_lsp.py" \
  --server-bin "$ROOT_DIR/$SERVER_BIN" \
  --all \
  --duration "$DURATION" \
  --docs "$DOCS" \
  --procs "$PROCS" \
  --startup-iterations "$STARTUP_ITERATIONS"

echo
echo "==> stress suite: PASS"
