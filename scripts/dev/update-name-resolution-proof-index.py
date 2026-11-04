#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Retired entry point for the former proof index."""


def main():
    raise SystemExit(
        "The legacy proof index is retired. Record new executions in evidence/tcl "
        "using docs/design/contracts/tcl-execution-evidence.md."
    )


if __name__ == "__main__":
    main()
