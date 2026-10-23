#!/bin/bash
set -u
source /tmp/tcl-lsp-naming-env.sh
exec python3 /tmp/compiler-postbatch-remaining33-v1/suite.py "$1"
