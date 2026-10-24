#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later

set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if [ "$#" -gt 1 ]; then
  echo "usage: validate.sh [--validate-only|--install|--verify-install]" >&2
  exit 2
fi
mode=--validate-only
if [ "$#" -eq 1 ]; then
  mode=$1
fi
exec python3 "$SCRIPT_DIR/validate.py" "$mode"
