#!/usr/bin/env bash
set -euo pipefail
export NH_VALIDATION_DIR=/tmp/vm-notom16-runtime-template-v1/validation
exec python3 /tmp/vm-notom16-runtime-template-v1/suite.py
