#!/bin/bash
set -euo pipefail
cd /workspace/naming-migration-active/tcl-lsp
source /tmp/tcl-lsp-naming-env.sh
exec python3 /tmp/ccg-diagnostic-three-v1/suite.py "$1"
