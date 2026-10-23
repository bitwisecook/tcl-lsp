#!/usr/bin/env bash
set -euo pipefail
source /tmp/tcl-lsp-naming-env.sh
export NH_VALIDATION_DIR=/tmp/lsp-body-minify-diagnostic-v3-source30-direct5/validation
exec python3 /tmp/lsp-body-minify-diagnostic-v3-source30-direct5/suite.py
