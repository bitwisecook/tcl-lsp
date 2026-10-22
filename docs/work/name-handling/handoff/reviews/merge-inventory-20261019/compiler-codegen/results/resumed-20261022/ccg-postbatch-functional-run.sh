#!/bin/bash
set -euo pipefail
cd /workspace/naming-migration-active/tcl-lsp
source /tmp/tcl-lsp-naming-env.sh
exec python3 /tmp/ccg-postbatch-functional-suite.py "${1:-/tmp/ccg-postbatch-functional-plan.json}"
