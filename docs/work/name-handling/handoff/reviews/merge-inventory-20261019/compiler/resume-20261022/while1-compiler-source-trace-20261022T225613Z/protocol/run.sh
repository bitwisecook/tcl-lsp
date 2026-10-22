#!/bin/bash
set -u
source /tmp/tcl-lsp-naming-env.sh
exec python3 /tmp/compiler-rbs-existing-trace-v1/suite.py "$1"
