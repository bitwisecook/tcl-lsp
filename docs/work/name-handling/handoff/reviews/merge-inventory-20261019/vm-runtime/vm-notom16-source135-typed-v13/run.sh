#!/usr/bin/env bash
set -euo pipefail
export NH_VALIDATION_DIR=/tmp/vm-notom16-source135-typed-v13/validation
exec python3 /tmp/vm-notom16-source135-typed-v13/suite.py
