#!/usr/bin/env bash
set -euo pipefail
export NH_VALIDATION_DIR=/tmp/vm26-source135-typed-v13/validation
exec python3 /tmp/vm26-source135-typed-v13/suite.py
