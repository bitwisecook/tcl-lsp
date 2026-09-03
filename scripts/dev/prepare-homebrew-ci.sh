#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# GitHub's macOS images can retain third-party taps that this repository never
# uses. Homebrew inspects every configured tap during a later `brew install`,
# and an untrusted aws/tap then emits a workflow annotation before the pinned
# Rust setup action can install bash. Remove only that exact unused tap; do not
# trust it or weaken Homebrew's tap-trust policy (issue #1684).

set -eu

if ! command -v brew >/dev/null 2>&1; then
    echo "prepare-homebrew-ci: brew is required on the macOS runner" >&2
    exit 1
fi

if brew tap | grep -Fxq 'aws/tap'; then
    echo "==> Removing unused runner Homebrew tap aws/tap"
    brew untap aws/tap
fi
