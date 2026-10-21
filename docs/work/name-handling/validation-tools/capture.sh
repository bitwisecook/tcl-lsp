#!/usr/bin/env bash
# Build the six LSP library test targets under source/provider guards, then run the 1,085-selector direct-LSP capture.
set -euo pipefail
N=${1:?usage: capture.sh <build-number> [capture-name]}
NAME=${2:-lsp-$N}
T=$(cd "$(dirname "$0")" && pwd)
R=$(git -C "$T" rev-parse --show-toplevel)
W=${NH_VALIDATION_DIR:-$R/tmp/name-handling-validation}
mkdir -p "$W" "$R/tmp/logs"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=$R/tmp/jimtcl-oracle/jimsh

python3 "$T/capture-inputs.py" "$W/pre-build$N-inputs.json"
python3 "$T/capture-providers.py" "$W/pre-build$N-providers.json"
set +e
(cd "$R" && cargo test --locked --no-run --message-format=json --lib --bins \
    -p tcl-lsp-core -p tcl-lsp-db -p tcl-lsp-server -p tcl-registry -p tcl-spectcl -p tcl-spec-studio) \
    >"$R/tmp/logs/build$N.jsonl" 2>"$R/tmp/logs/build$N.stderr"
echo "build exit $?" >"$R/tmp/logs/build$N.exit"
set -e
python3 "$T/finalize-build.py" "$N"
python3 "$T/run-lsp.py" "$NAME" "lsp-build-$N"
