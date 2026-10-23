#!/usr/bin/env bash
set -euo pipefail
export NH_VALIDATION_DIR=/tmp/vm26-latebound-template-v2/validation
exec python3 /tmp/vm26-latebound-template-v2/suite.py
