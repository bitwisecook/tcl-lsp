#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
rustc --edition=2024 --target wasm32-unknown-unknown --crate-type cdylib \
  -C opt-level=z -C lto -C panic=abort -C strip=symbols \
  network_wasm.rs -o src/network/address.wasm
