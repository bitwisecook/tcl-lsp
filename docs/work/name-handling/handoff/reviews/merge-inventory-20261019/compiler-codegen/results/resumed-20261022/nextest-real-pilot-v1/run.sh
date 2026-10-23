#!/bin/bash
set -euo pipefail
source /tmp/tcl-lsp-naming-env.sh
export CARGO_TARGET_DIR=/tmp/ccg-nextest-real-pilot-v1/target
exec python3 /tmp/ccg-nextest-real-pilot-v1/runner.py "$@"
