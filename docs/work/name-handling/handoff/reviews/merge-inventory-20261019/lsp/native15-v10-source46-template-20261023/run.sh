#!/usr/bin/env bash
set -euo pipefail
source /tmp/tcl-lsp-naming-env.sh
export NH_VALIDATION_DIR=/tmp/lsp-native-protocol-v10-source46-direct15-template/validation
exec python3 /tmp/lsp-native-protocol-v10-source46-direct15-template/suite.py
